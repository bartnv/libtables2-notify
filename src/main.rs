extern crate bus;
extern crate postgres;
extern crate fallible_iterator;
extern crate websocket;
extern crate openssl;

use std::thread;
use std::process::exit;
use std::sync::{Arc,Mutex};
use bus::Bus;
use fallible_iterator::FallibleIterator;
use websocket::{Server, Message};
use openssl::ssl::{SslMethod, SslAcceptorBuilder};
use openssl::x509;

fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    println!("Not enough arguments");
    exit(-1);
  }

  let mut builder = SslAcceptorBuilder::mozilla_modern_raw(SslMethod::tls()).unwrap();
  {
    let context = builder.builder_mut();
    let _ = context.set_private_key_file("privkey.pem", x509::X509_FILETYPE_PEM);
    let _ = context.set_certificate_chain_file("fullchain.pem");
  }
  let acceptor = builder.build();

  let server = Server::bind_secure("0.0.0.0:6185", acceptor).unwrap_or_else(|e| { println!("Failed to bind to listen address: {}", e.to_string()); exit(1); });
  let bus = Arc::new(Mutex::new(Bus::new(10)));
  let bus_server = bus.clone();
  let mut conncount = 0;
  thread::spawn(move || {
    for res in server {
      conncount += 1;
      match res {
        Ok(res) => {
          let mut stream = res.accept().unwrap();
          let addr = stream.peer_addr().unwrap();
          let mut receiver = bus_server.lock().unwrap().add_rx();
          println!("New connection #{} from {}", conncount, addr);
          thread::spawn(move || {
            loop {
              let line: String = receiver.recv().unwrap();
              let message: Message = Message::text(line);
              match stream.send_message(&message) {
                Ok(_) => {
                    println!("#{} sent message {}", conncount, String::from_utf8(message.payload.to_vec()).unwrap());
                }
                Err(e) => {
                  println!("Error from {}: {}", addr, e.to_string());
                  break;
                }
              }
            }
          });
        }
        Err(e) => println!("Error: {:?}", e.error)
      }
    }
  });

  let conn = postgres::Connection::connect("postgres://web:j023th4hgfuorwegfvp9rhwefv@localhost/romancities", postgres::TlsMode::None).unwrap();
  let mut query = String::from("LISTEN ");
  query.push_str(&args[1]);
  conn.execute(&query, &[]).unwrap();
  let notif = conn.notifications();
  let mut notifications = notif.blocking_iter();
  println!("Ready to receive notifications");
  loop {
    let res = notifications.next();
    match res {
      Ok(res) => {
        let notif = res.unwrap();
        println!("{:?}", notif);
        match bus.lock().unwrap().try_broadcast(notif.payload) {
            Ok(_) => {}
            Err(_) => println!("try_broadcast failed")
        }
      }
      Err(e) => println!("Error: {}", e.to_string())
    }
  }
}
