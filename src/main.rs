use std::{
    format,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    println,
};

use anyhow::{anyhow, Context, Result};

enum HttpMethod {
    Get,
    Post,
}

struct HttpRequest {
    method: HttpMethod,
    path: String,
    version: String,
}

#[derive(Debug)]
struct HttpResponse {
    version: String,
    code: i32,
    msg: String,
}

fn handle(req: HttpRequest) -> HttpResponse {
    if req.path == "/" {
        HttpResponse {
            version: req.version,
            code: 200,
            msg: "OK".to_owned(),
        }
    } else {
        HttpResponse {
            version: req.version,
            code: 404,
            msg: "Not Found".to_owned(),
        }
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<Vec<String>> {
    let mut result = vec![];
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                line = line
                    .strip_suffix("\r\n")
                    .ok_or(anyhow!("Unexpected suffix"))?
                    .to_owned();
                if line.is_empty() {
                    break;
                }
                result.push(line);
            }
            _ => return Err(anyhow!("Failed reading http request")),
        };
    }
    Ok(result)
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let http_req_lines = read_http_request(stream)?;
    let mut start_line = http_req_lines
        .iter()
        .next()
        .context("start_line not found")?
        .split(" ");
    let method = start_line.next().context("method not found")?.to_owned();
    let method = if method == "GET" {
        HttpMethod::Get
    } else if method == "POST" {
        HttpMethod::Post
    } else {
        return Err(anyhow!("Unexpected method"));
    };
    let path = start_line.next().context("path not found")?.to_owned();
    let version = start_line.next().context("version not found")?.to_owned();
    Ok(HttpRequest {
        method,
        path,
        version,
    })
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> Result<()> {
    Ok(stream.write_all(
        format!(
            "{} {} {}\r\n\r\n",
            response.version, response.code, response.msg
        )
        .as_bytes(),
    )?)
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                let req = read_request(&mut stream).expect("Failed reading request");
                let res = handle(req);
                write_response(&mut stream, res).expect("Failed writing response");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
