use std::io::Read;
use std::net::TcpListener;
use std::net::TcpStream;
use std::{
    collections::HashMap,
    env,
    io::{self, BufRead, Write},
};
use std::{fs, thread};

use flate2::write::GzEncoder;
use flate2::Compression;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let args: Vec<String> = env::args().collect();

    let mut directory = String::new();
    for i in 0..args.len() {
        if args[i] == "--directory" && i + 1 < args.len() {
            directory = args[i + 1].clone()
        }
    }

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let dir = directory.clone();

        // we should be using a threadpool instead of creating a new thread for each connection.
        // See https://doc.rust-lang.org/book/ch21-02-multithreaded.html for how to implement one
        thread::spawn(move || {
            handle_connection(stream, dir).unwrap();
        });
    }
}

/// Handles incoming connections by responding with a 200 status code.
fn handle_connection(stream: TcpStream, directory: String) -> io::Result<()> {
    let mut reader = io::BufReader::new(&stream);

    loop {
        let request = parse_request(&mut reader)?;
        let close_conn = request.headers.get("Connection") == Some(&"close".to_string());
        let mut response_headers = vec![];
        if close_conn {
            response_headers.push("Connection: close\r\n");
        }

        match request.path.as_str() {
            "/" => write_response(&stream, "200 OK", None, response_headers)?,
            s if s.starts_with("/echo/") => {
                let requested_encoding = request
                    .headers
                    .get("Accept-Encoding")
                    .map(|s| s.as_str())
                    .unwrap_or_default();

                let resp_echo = if requested_encoding.contains("gzip") {
                    response_headers.push("Content-Encoding: gzip\r\n");
                    &compress_string(s.strip_prefix("/echo/").unwrap_or_default())?
                } else {
                    s.strip_prefix("/echo/").unwrap_or_default().as_bytes()
                };

                write_response(&stream, "200 OK", Some(resp_echo), response_headers)?
            }
            "/user-agent" => write_response(
                &stream,
                "200 OK",
                request.headers.get("User-Agent").map(|s| s.as_bytes()),
                response_headers,
            )?,
            s if s.starts_with("/files/") => {
                let file_name = s.strip_prefix("/files/").unwrap();
                let fp = format!("{}/{}", directory, file_name);

                match request.method.as_str() {
                    "GET" => match fs::read_to_string(fp) {
                        Ok(contents) => {
                            response_headers.push("Content-Type: application/octet-stream\r\n");
                            write_response(
                                &stream,
                                "200 OK",
                                Some(contents.as_bytes()),
                                response_headers,
                            )?
                        }
                        Err(_) => write_response(&stream, "404 Not Found", None, response_headers)?,
                    },
                    "POST" => {
                        println!("body: {}", request.body);
                        fs::write(fp, request.body)?;
                        println!("wrote new file");
                        write_response(&stream, "201 Created", None, response_headers)?
                    }
                    _ => write_response(&stream, "404 Not Found", None, response_headers)?,
                }
            }
            _ => write_response(&stream, "404 Not Found", None, response_headers)?,
        }
        if close_conn {
            return Ok(());
        }
    }
}

fn write_response(
    mut stream: &TcpStream,
    status: &str,
    body: Option<&[u8]>,
    headers: Vec<&str>,
) -> io::Result<()> {
    let status_line = format!("HTTP/1.1 {}\r\n", status);
    stream.write_all(status_line.as_bytes())?;
    println!("response headers: {:?}", headers);

    if let Some(txt) = body {
        stream.write_all(
            format!(
                "{}Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                headers.join(""),
                txt.len()
            )
            .as_bytes(),
        )?;
        stream.write_all(txt)?;
    } else {
        stream.write_all("\r\n".as_bytes())?;
    }
    stream.flush()?;

    Ok(())
}

fn parse_request(reader: &mut io::BufReader<&TcpStream>) -> io::Result<Request> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    println!("request line {}", request_line);
    let request_line_parts: Vec<_> = request_line.trim().split_whitespace().collect();

    let headers = parse_headers(reader)?;

    let body = get_body(reader, headers.get("Content-Length"))?;

    Ok(Request {
        method: request_line_parts[0].to_string(),
        path: request_line_parts[1].to_string(),
        headers,
        body,
    })
}

fn parse_headers(reader: &mut io::BufReader<&TcpStream>) -> io::Result<HashMap<String, String>> {
    let mut request_headers: HashMap<String, String> = HashMap::new();
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }
        let mut parts = line.splitn(2, ":");
        if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
            println!("{}: {}", name, value);
            request_headers.insert(name.to_string(), value.trim().to_string());
        }
    }

    Ok(request_headers)
}

fn get_body(
    reader: &mut io::BufReader<&TcpStream>,
    content_length_header: Option<&String>,
) -> io::Result<String> {
    let content_length = content_length_header
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body_bytes = vec![0u8; content_length];
    reader.read_exact(&mut body_bytes)?;
    Ok(String::from_utf8_lossy(&body_bytes).to_string())
}

struct Request {
    path: String,
    method: String,
    headers: HashMap<String, String>,
    body: String,
}

fn compress_string(s: &str) -> io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(s.as_bytes())?;
    encoder.finish()
}
