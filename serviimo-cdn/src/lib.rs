//! Local HTTP server for the generated server list and optional static content.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use apiimo_network::{LocalDir, ServerGroup, ServerListPublish};

const MAX_HEADER_BYTES: usize = 16 * 1024;

pub fn server_list_document(gate_address: &str) -> ServerListPublish {
    let mut groups = BTreeMap::new();
    groups.insert(
        "Aniimo-OS".to_string(),
        ServerGroup {
            enable_default_server_detect: Some(true),
            external_addr_list: None,
            ms_gate_addr_list: Some(vec![gate_address.to_string()]),
            local_dir_data: vec![LocalDir {
                cluster_name: "Serviimo".to_string(),
                cluster_id: 1,
                external_addr_list: None,
                ms_gate_addr_list: None,
            }],
        },
    );
    ServerListPublish {
        server_group_data: groups,
    }
}

pub fn server_list_json(gate_address: &str) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(&server_list_document(gate_address))
}

pub fn serve(
    address: &str,
    document_root: &Path,
    gate_address: &str,
    shutdown: Arc<AtomicBool>,
) -> io::Result<()> {
    fs::create_dir_all(document_root)?;
    let document =
        server_list_json(gate_address).map_err(|error| io::Error::other(error.to_string()))?;
    fs::write(document_root.join("server_list_publish.json"), &document)?;

    let listener = TcpListener::bind(address)?;
    listener.set_nonblocking(true)?;
    println!("serviimo-cdn listening on {}", listener.local_addr()?);
    let root = document_root.canonicalize()?;

    while shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                let root = root.clone();
                let document = document.clone();
                thread::Builder::new()
                    .name("serviimo-cdn-client".into())
                    .spawn(move || {
                        if let Err(error) = handle_request(stream, &root, &document) {
                            eprintln!("serviimo-cdn request ended: {}", error);
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

fn handle_request(mut stream: TcpStream, root: &Path, server_list: &[u8]) -> io::Result<()> {
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

    if method != "GET" && method != "HEAD" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"method not allowed\n",
        );
    }
    if path == "/server_list_publish.json"
        || path == "/server_list/server_list_publish.json"
        || path == "/"
    {
        return respond(&mut stream, "200 OK", "application/json", server_list);
    }

    match static_file_path(root, path) {
        Some(file_path) => match fs::read(&file_path) {
            Ok(body) => {
                let content_type = content_type(&file_path);
                respond(&mut stream, "200 OK", content_type, &body)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                respond(&mut stream, "404 Not Found", "text/plain", b"not found\n")
            }
            Err(error) => Err(error),
        },
        None => respond(&mut stream, "404 Not Found", "text/plain", b"not found\n"),
    }
}

fn static_file_path(root: &Path, request_path: &str) -> Option<PathBuf> {
    let relative = request_path.trim_start_matches('/');
    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let candidate = root.join(relative_path);
    candidate.is_file().then_some(candidate)
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
    {
        "json" => "application/json",
        "xdf" => "application/octet-stream",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
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
    if status.starts_with("200") {
        stream.write_all(body)?;
    }
    stream.flush()
}
