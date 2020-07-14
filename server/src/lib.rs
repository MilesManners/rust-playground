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
                println!("Shutting down worker {}", worker.id);

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
                        println!("Worker {} got a job; executing.", id);

                        job();
                    }
                    Message::Terminate => {
                        println!("Worker {} was told to terminate.", id);

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
