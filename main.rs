use std::fs;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;
use infer;
use url_escape;

fn handle_request(mut stream: TcpStream, root_dir: &PathBuf) {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();
    
    // Convert request into a string
    let request = String::from_utf8_lossy(&buffer[..]);

    // Extract the requested file path (basic HTTP parsing)
    let request_line = request.lines().next().unwrap();
    let path = request_line.split_whitespace().nth(1).unwrap();

    // Ensure CJK characters and special symbols are handled
    let decoded_path = url_escape::decode_component(path).unwrap();

    // Build the full file path
    let mut full_path = root_dir.clone();
    full_path.push(decoded_path.trim_start_matches('/'));

    // Ensure we stay inside the root directory (backtracking prevention)
    let rootcwd = root_dir.canonicalize().unwrap();
    let resource = rootcwd.join(&full_path);
    if resource.canonicalize().unwrap().starts_with(&rootcwd) {
        if resource.is_dir() {
            // If it's a directory, list contents
            serve_directory(stream, &resource);
        } else if resource.is_file() {
            // If it's a file, serve the file
            serve_file(stream, &resource);
        }
    } else {
        send_404(stream);
    }
}

fn serve_directory(mut stream: TcpStream, dir: &PathBuf) {
    let mut body = String::new();
    for entry in WalkDir::new(dir).max_depth(1) {
        let entry = entry.unwrap();
        let path = entry.path().display().to_string();
        body.push_str(&format!("<a href=\"{}\">{}</a><br>", path, path));
    }

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}{}{}",
        "<html><body>", body, "</body></html>"
    );
    stream.write_all(response.as_bytes()).unwrap();
}

fn serve_file(mut stream: TcpStream, file_path: &PathBuf) {
    let content = fs::read(file_path).unwrap();

    // Detect the file type to set the appropriate content-type header
    let mime_type = if let Some(kind) = infer::get(&content) {
        kind.mime_type()
    } else {
        "application/octet-stream"
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n\r\n", mime_type
    );
    stream.write_all(response.as_bytes()).unwrap();
    stream.write_all(&content).unwrap();
}

fn send_404(mut stream: TcpStream) {
    let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
    stream.write_all(response.as_bytes()).unwrap();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let root_dir = std::env::current_dir().unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_request(stream, &root_dir);
    }
}
