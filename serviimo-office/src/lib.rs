//! Small HTTP stubs for the local account bootstrap and profiling endpoints.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

const MAX_HEADER_BYTES: usize = 16 * 1024;

pub fn serve(address: &str, shutdown: Arc<AtomicBool>) -> io::Result<()> {
    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    println!("serviimo-office listening on {}", listener.local_addr()?);

    while shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                thread::Builder::new()
                    .name("serviimo-office-client".into())
                    .spawn(move || {
                        if let Err(error) = handle_request(stream) {
                            eprintln!("serviimo-office request ended: {}", error);
                        }
                    })?;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn handle_request(mut stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while request.len() < MAX_HEADER_BYTES && !request.windows(4).any(|part| part == b"\r\n\r\n") {
        let count = stream.read(&mut chunk)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request",
            ));
        }
        request.extend_from_slice(&chunk[..count]);
    }
    if !request.windows(4).any(|part| part == b"\r\n\r\n") {
        return respond(
            &mut stream,
            "431 Request Header Fields Too Large",
            "text/plain",
            b"headers too large\n",
        );
    }

    let first_line = request
        .split(|byte| *byte == b'\n')
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let request_line = String::from_utf8_lossy(first_line);
    let line = request_line.trim();
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let path = target.split('?').next().unwrap_or("");

    let (status, body) = match path {
        "/api/init" => (
            "200 OK",
            br#"{"status":1,"data":{"server":"serviimo","local":true}}"#.as_slice(),
        ),
        "/profile-sessions/start" => (
            "200 OK",
            br#"{"status":1,"sessionId":"local-profile-session","uploadUrl":""}"#.as_slice(),
        ),
        _ => (
            "404 Not Found",
            b"{\"status\":0,\"message\":\"not found\"}".as_slice(),
        ),
    };
    let content_type = if method == "GET" || method == "POST" {
        "application/json"
    } else {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "application/json",
            b"{\"status\":0,\"message\":\"method not allowed\"}",
        );
    };
    respond(&mut stream, status, content_type, body)
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}
