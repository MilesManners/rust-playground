use http::{Method, Request, Response, Version};
use lazy_static::lazy_static;
use regex::bytes::Regex;
use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::prelude::*,
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
};

use crate::thread_pool::ThreadPool;

/// Static routing is looked up from a hashmap.
type Routes = HashMap<String, String>;

/// A very simple multi-threaded web server with static routing.
pub struct WebServer {
    thread_limit: usize,
    routes: Arc<RwLock<Routes>>,
}

impl WebServer {
    /// Creates a new web server.
    ///
    /// Routes cannot be changed once the server is started
    pub fn new(thread_limit: usize, routes: Routes) -> WebServer {
        let routes = Arc::new(RwLock::new(routes));

        WebServer {
            thread_limit,
            routes,
        }
    }

    /// Starts the web server.
    pub fn start(&self, ip: &str) -> Result<(), Box<dyn Error>> {
        // Create a listener on the address we want to respond to
        let listener = TcpListener::bind(ip)?;
        // Create a pool of threads to prevent the server from blocking
        let pool = ThreadPool::new(self.thread_limit)?;

        // Start listening
        for stream in listener.incoming() {
            let stream = stream?;

            let routes = Arc::clone(&self.routes);

            // Pass handling of the connection off to a seperate thread
            pool.execute(|| {
                handle_connection(routes, stream).unwrap();
            })
        }

        Ok(())
    }
}

/// Handles an individual connection.
///
/// Performed by threads.
fn handle_connection(
    routes: Arc<RwLock<Routes>>,
    mut stream: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 512];
    stream.read(&mut buffer)?;

    let request = parse_request(&buffer);

    // Pass on the request
    let response = response(routes, request).unwrap();

    // Parse the response back into a format we can send back
    let response = format!(
        "{:?} {}\r\n\r\n{}",
        response.version(),
        response.status(),
        response.body()
    );

    // Send the response back
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();

    Ok(())
}

fn parse_request(buffer: &[u8]) -> Request<&[u8]> {
    lazy_static! {
        static ref LINES: Regex = Regex::new(r"(.*?)\r?\n").unwrap();
    }

    let mut lines = LINES.find_iter(buffer);

    // Parse the first line
    let first_line: &[u8] = &buffer[lines.next().unwrap().range()];

    lazy_static! {
        static ref TOKENS: Regex = Regex::new(r"(.*?)\s").unwrap();
    }

    let mut tokens = TOKENS.find_iter(first_line);

    let method: &[u8] = &first_line[tokens.next().unwrap().range()];
    let uri: &[u8] = &first_line[tokens.next().unwrap().range()];
    let version: &[u8] = &first_line[tokens.next().unwrap().range()];

    println!("{}", String::from_utf8_lossy(version));

    let version = match version {
        b"HTTP/0.9" => Version::HTTP_09,
        b"HTTP/1.0" => Version::HTTP_10,
        b"HTTP/1.1" => Version::HTTP_11,
        b"HTTP/2.0" => Version::HTTP_2,
        b"HTTP/3.0" => Version::HTTP_3,
        _ => unreachable!(),
    };

    // Start building the request with the information we have so far
    let mut request = Request::builder().method(method).uri(uri).version(version);

    // Store the regex for headers statically to save processing time
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?P<key>.*?):(?P<value>.*)").unwrap();
    }

    // Parse the headers
    loop {
        match lines.next() {
            None => break,
            Some(line) => {
                let range = line.range();

                if range.start == range.end {
                    break;
                }

                let header = RE.captures(&buffer[range]).unwrap();

                request = request.header(&header["key"], &header["value"]);
            }
        }
    }

    // Build the body from the remaining lines
    let body = match lines.next() {
        None => &buffer[0..0],
        Some(line) => &buffer[line.start()..],
    };

    // Turn the body back into bytes
    request.body(body).unwrap()
}

fn response(
    routes: Arc<RwLock<Routes>>,
    request: Request<&[u8]>,
) -> http::Result<Response<String>> {
    let method = request.method();

    match *method {
        Method::GET | Method::POST => match routes.read().unwrap().get(request.uri().path()) {
            Some(file) => {
                let body = fs::read_to_string(file).unwrap();

                Response::builder().status(200).body(body)
            }
            None => {
                let body = fs::read_to_string("404.html").unwrap();

                Response::builder().status(404).body(body)
            }
        },
        Method::HEAD | Method::OPTIONS => Response::builder()
            .status(501)
            .body(format!("Server does not support {} requests", method)),
        _ => Response::builder()
            .status(405)
            .body(format!("Server does not allow {} requests", method)),
    }
}
