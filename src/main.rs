use std::io::{self, BufRead, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
use std::net::TcpStream;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");
                handle_connection(_stream).unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

/// Handles incoming connections by responding with a 200 status code.
fn handle_connection(stream: TcpStream) -> io::Result<()> {
    let mut reader = io::BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let request_line_parts: Vec<_> = line.split(" ").collect();
    let path = request_line_parts[1];

    match path {
        "/" => write_response(stream, "200 OK", None)?,
        s if s.starts_with("/echo/") => {
            let resp_echo = s.strip_prefix("/echo/");
            write_response(stream, "200 OK", resp_echo)?
        }
        _ => write_response(stream, "404 Not Found", None)?,
    }
    // stream.write_all(status_line)?;

    // stream.flush()?;

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
