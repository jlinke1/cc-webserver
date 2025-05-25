#[allow(unused_imports)]
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        thread::spawn(|| {
            handle_connection(stream).unwrap();
        });
    }
}

/// Handles incoming connections by responding with a 200 status code.
fn handle_connection(stream: TcpStream) -> io::Result<()> {
    let mut reader = io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let request_line_parts: Vec<_> = request_line.split(" ").collect();
    let path = request_line_parts[1];

    let mut request_headers: HashMap<String, String> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let mut parts = line.splitn(2, ":");
        if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
            println!("{}: {}", name, value);
            request_headers.insert(name.to_string(), value.trim().to_string());
        } else {
            break;
        }
    }

    match path {
        "/" => write_response(stream, "200 OK", None)?,
        s if s.starts_with("/echo/") => {
            let resp_echo = s.strip_prefix("/echo/");
            write_response(stream, "200 OK", resp_echo)?
        }
        "/user-agent" => write_response(
            stream,
            "200 OK",
            request_headers.get("User-Agent").map(|s| s.as_str()),
        )?,
        _ => write_response(stream, "404 Not Found", None)?,
    }

    Ok(())
}

fn write_response(mut stream: TcpStream, status: &str, body: Option<&str>) -> io::Result<()> {
    let status_line = format!("HTTP/1.1 {}\r\n", status);
    stream.write_all(status_line.as_bytes())?;

    if let Some(txt) = body {
        let headers = format!(
            "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
            txt.len()
        );
        stream.write_all(headers.as_bytes())?;
        stream.write_all(txt.as_bytes())?;
        stream.write_all("\r\n".as_bytes())?;
    } else {
        stream.write_all("\r\n".as_bytes())?;
    }
    stream.flush()?;

    Ok(())
}
