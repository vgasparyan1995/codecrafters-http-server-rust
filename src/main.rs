use std::{io::Write, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                stream.write("HTTP/1.1 200 OK\r\n\r\n".as_bytes()).expect("Failed on 200");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
