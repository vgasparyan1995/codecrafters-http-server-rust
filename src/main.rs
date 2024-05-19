use std::{
    collections::HashMap,
    format,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    println,
};

use anyhow::{anyhow, Context, Result};
use itertools::Itertools;

const CODE_200_OK: &str = "200 OK";
const CODE_400_BAD_REQUEST: &str = "400 Bad Request";
const CODE_404_NOT_FOUND: &str = "404 Not Found";

enum HttpMethod {
    Get,
    Post,
}

struct HttpRequest {
    method: HttpMethod,
    path: String,
    version: String,
    headers: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct HttpResponse {
    version: String,
    code: &'static str,
    headers: HashMap<String, String>,
    content: String,
}

impl HttpResponse {
    fn in_response_to(mut self, req: &HttpRequest) -> HttpResponse {
        self.version = req.version.clone();
        self
    }

    fn with_code(mut self, code: &'static str) -> HttpResponse {
        self.code = code;
        self
    }

    fn with_content(mut self, content: &str) -> HttpResponse {
        self.content = content.to_string();
        self.headers
            .insert("Content-Type".to_owned(), "text/plain".to_owned());
        self.headers
            .insert("Content-Length".to_owned(), content.len().to_string());
        self
    }
}

fn handle(req: HttpRequest) -> HttpResponse {
    match req.method {
        HttpMethod::Get => handle_get(req),
        HttpMethod::Post => handle_post(req),
    }
}

fn handle_get(req: HttpRequest) -> HttpResponse {
    let rsp = HttpResponse::default().in_response_to(&req);
    if req.path == "/" {
        return rsp.with_code(CODE_200_OK);
    }

    if req.path.starts_with("/echo/") {
        return handle_echo(req);
    }

    if req.path == "/user-agent" {
        return handle_user_agent(req);
    }

    rsp.with_code(CODE_404_NOT_FOUND)
}

fn handle_post(req: HttpRequest) -> HttpResponse {
    HttpResponse::default()
        .in_response_to(&req)
        .with_code(CODE_404_NOT_FOUND)
}

fn handle_echo(req: HttpRequest) -> HttpResponse {
    match req.path.strip_prefix("/echo/") {
        Some(msg) => HttpResponse::default()
            .in_response_to(&req)
            .with_code(CODE_200_OK)
            .with_content(msg),
        None => HttpResponse::default()
            .in_response_to(&req)
            .with_code(CODE_400_BAD_REQUEST),
    }
}

fn handle_user_agent(req: HttpRequest) -> HttpResponse {
    match req.headers.get(&"User-Agent".to_owned()) {
        Some(user_agent) => HttpResponse::default()
            .in_response_to(&req)
            .with_code(CODE_200_OK)
            .with_content(user_agent),
        None => HttpResponse::default()
            .in_response_to(&req)
            .with_code(CODE_400_BAD_REQUEST),
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
    let headers = http_req_lines
        .iter()
        .skip(1)
        .filter_map(|line| {
            line.split_once(": ")
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
        })
        .collect::<HashMap<_, _>>();
    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
    })
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> Result<()> {
    let version = response.version;
    let code = response.code;
    let headers = response
        .headers
        .into_iter()
        .map(|(k, v)| format!("{k}: {v}\r\n"))
        .join("");
    let content = response.content;
    Ok(stream.write_all(
        format!(
            "{version} {code}\r\n\
            {headers}\r\n\
            {content}"
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
