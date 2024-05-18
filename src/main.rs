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

fn handle(req: HttpRequest) -> HttpResponse {
    match req.method {
        HttpMethod::Get => handle_get(req),
        HttpMethod::Post => handle_post(req),
    }
}

fn handle_get(req: HttpRequest) -> HttpResponse {
    if req.path == "/" {
        return HttpResponse {
            version: req.version,
            code: CODE_200_OK,
            ..Default::default()
        };
    }

    if req.path.starts_with("/echo/") {
        return handle_echo(req);
    }

    if req.path == "/user-agent" {
        return handle_user_agent(req);
    }

    HttpResponse {
        version: req.version,
        code: CODE_404_NOT_FOUND,
        ..Default::default()
    }
}

fn handle_post(req: HttpRequest) -> HttpResponse {
    HttpResponse {
        version: req.version,
        code: CODE_404_NOT_FOUND,
        ..Default::default()
    }
}

fn handle_echo(req: HttpRequest) -> HttpResponse {
    match req.path.strip_prefix("/echo/") {
        Some(msg) => HttpResponse {
            version: req.version,
            code: CODE_200_OK,
            headers: [
                ("Content-Type".to_owned(), "text/plain".to_owned()),
                ("Content-Length".to_owned(), msg.len().to_string()),
            ]
            .into_iter()
            .collect(),
            content: msg.to_owned(),
        },
        None => HttpResponse {
            version: req.version,
            code: CODE_400_BAD_REQUEST,
            ..Default::default()
        },
    }
}

fn handle_user_agent(req: HttpRequest) -> HttpResponse {
    match req.headers.get(&"User-Agent".to_owned()) {
        Some(user_agent) => HttpResponse {
            version: req.version,
            code: CODE_200_OK,
            headers: [
                ("Content-Type".to_owned(), "text/plain".to_owned()),
                ("Content-Length".to_owned(), user_agent.len().to_string()),
            ]
            .into_iter()
            .collect(),
            content: user_agent.clone(),
        },
        None => HttpResponse {
            version: req.version,
            code: CODE_400_BAD_REQUEST,
            ..Default::default()
        },
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
