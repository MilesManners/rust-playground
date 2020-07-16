use server::web_server::WebServer;
use std::collections::HashMap;

const THREAD_LIMIT: usize = 4;

fn main() {
    let routes: HashMap<String, String> = [(String::from("/"), String::from("hello.html"))]
        .iter()
        .cloned()
        .collect();

    let server = WebServer::new(THREAD_LIMIT, routes);

    server.start("127.0.0.1:7878").unwrap();
}
