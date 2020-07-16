pub mod web_server {
    use http::{Method, Request, Response, Version};
    use std::{
        collections::HashMap,
        error::Error,
        fs,
        io::prelude::*,
        net::{TcpListener, TcpStream},
        sync::{Arc, RwLock},
    };

    use crate::thread_pool::ThreadPool;

    type Routes = HashMap<String, String>;

    pub struct WebServer {
        thread_limit: usize,
        routes: Arc<RwLock<Routes>>,
    }

    impl WebServer {
        pub fn new(thread_limit: usize, routes: Routes) -> WebServer {
            let routes = Arc::new(RwLock::new(routes));

            WebServer {
                thread_limit,
                routes,
            }
        }

        pub fn start(&self, ip: &str) -> Result<(), Box<dyn Error>> {
            let listener = TcpListener::bind(ip)?;
            let pool = ThreadPool::new(self.thread_limit)?;

            for stream in listener.incoming() {
                let stream = stream?;

                let routes = Arc::clone(&self.routes);

                pool.execute(|| {
                    handle_connection(routes, stream).unwrap();
                })
            }

            Ok(())
        }
    }

    fn handle_connection(
        routes: Arc<RwLock<Routes>>,
        mut stream: TcpStream,
    ) -> Result<(), Box<dyn Error>> {
        let mut buffer = [0; 512];
        stream.read(&mut buffer)?;

        println!("{}\n", String::from_utf8_lossy(&buffer));

        let lines: Vec<String> = buffer.lines().map(|line| line.unwrap()).collect();

        let (method, uri, version) = match lines
            .get(0)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .get(0..2)
            .unwrap()
        {
            &[a, b, c] => (a, b, c),
            _ => unreachable!(),
        };

        let method = Method::from_bytes(method.as_bytes()).unwrap();

        let version = match version {
            "HTTP/0.9" => Version::HTTP_09,
            "HTTP/1.0" => Version::HTTP_10,
            "HTTP/1.1" => Version::HTTP_11,
            "HTTP/2.0" => Version::HTTP_2,
            "HTTP/3.0" => Version::HTTP_3,
            _ => unreachable!(),
        };

        let request = Request::builder()
            .method(method)
            .uri(uri)
            .version(version)
            .body(vec![])
            .unwrap();

        let response = response(routes, request).unwrap();

        let response = format!(
            "{:?} {}\r\n\r\n{}",
            response.version(),
            response.status(),
            response.body()
        );

        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();

        Ok(())
    }

    fn response(
        routes: Arc<RwLock<Routes>>,
        request: Request<Vec<u8>>,
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
}

pub mod thread_pool {
    use std::{
        error::Error,
        fmt,
        sync::{mpsc, Arc, Mutex},
        thread,
    };

    type Job = Box<dyn FnOnce() + Send + 'static>;

    enum Message {
        NewJob(Job),
        Terminate,
    }

    pub struct ThreadPool {
        workers: Vec<Worker>,
        sender: mpsc::Sender<Message>,
    }

    impl ThreadPool {
        /// Creates a new ThreadPool.
        ///
        /// The size is the number of workers in the pool.
        ///
        /// # Panics
        ///
        /// The `new` function will panic if the size is zero.
        pub fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
            match size {
                0 => Err(PoolCreationError),
                _ => {
                    let (sender, receiver) = mpsc::channel();

                    let receiver = Arc::new(Mutex::new(receiver));

                    let workers = (0..size)
                        .map(|id| Worker::new(id, Arc::clone(&receiver)))
                        .collect();

                    Ok(ThreadPool { workers, sender })
                }
            }
        }

        pub fn execute<F>(&self, f: F)
        where
            F: FnOnce() + Send + 'static,
        {
            let job = Box::new(f);

            self.sender.send(Message::NewJob(job)).unwrap();
        }
    }

    impl Drop for ThreadPool {
        fn drop(&mut self) {
            println!("Sending terminate message to all workers.");

            for _ in &self.workers {
                self.sender.send(Message::Terminate).unwrap();
            }

            println!("Shutting down all workers.");

            for worker in &mut self.workers {
                // println!("Shutting down worker {}", worker.id);

                if let Some(thread) = worker.thread.take() {
                    thread.join().unwrap();
                }
            }
        }
    }

    struct Worker {
        id: usize,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl Worker {
        fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
            let thread = thread::spawn(move || loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => {
                        // println!("Worker {} got a job; executing.", id);

                        job();
                    }
                    Message::Terminate => {
                        // println!("Worker {} was told to terminate.", id);

                        break;
                    }
                }
            });

            Worker {
                id,
                thread: Some(thread),
            }
        }
    }

    #[derive(Debug)]
    pub struct PoolCreationError;

    impl fmt::Display for PoolCreationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Pool cannot be created with 0 threads")
        }
    }

    impl Error for PoolCreationError {}
}
