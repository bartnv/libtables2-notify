extern crate chan;
extern crate postgres;
extern crate fallible_iterator;

use std::thread;
use std::process::exit;
use std::io::Write;
use std::net::TcpListener;
use fallible_iterator::FallibleIterator;

fn main() {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    println!("Not enough arguments");
    exit(-1);
  }

  let server = TcpListener::bind("0.0.0.0:6185").unwrap_or_else(|e| { println!("Failed to bind to listen address: {}", e.to_string()); exit(1); });
  let (send, receive) = chan::async();
  thread::spawn(move || {
    for res in server.incoming() {
      let mut stream = res.unwrap();
      let addr = stream.peer_addr().unwrap();
      let receiver = receive.clone();
      println!("New incoming connection from {}", addr);
      thread::spawn(move || {
        loop {
          let line: String = receiver.recv().unwrap();
          stream.write(line.as_bytes());
        }
      });
    }
  });


  let conn = postgres::Connection::connect("postgres://web:j023th4hgfuorwegfvp9rhwefv@localhost/romancities", postgres::TlsMode::None).unwrap();
  let mut query = String::from("LISTEN ");
  query.push_str(&args[1]);
  conn.execute(&query, &[]).unwrap();
  let notifications = conn.notifications();
  let mut iter = notifications.blocking_iter();
  println!("Ready to receive notifications");
  loop {
    let res = iter.next();
    match res {
      Ok(res) => {
        let mut notif = res.unwrap();
        println!("{:?}", notif);
        notif.payload.push('\n');
        send.send(notif.payload);
      }
      Err(e) => println!("Error: {}", e.to_string())
    }
  }
}
