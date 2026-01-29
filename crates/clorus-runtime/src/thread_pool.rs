/// Thread pool for agent execution
///
/// Provides fixed-size pool for CPU-bound operations (send)
/// and expandable pool for I/O-bound operations (send-off)

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A simple thread pool
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Create a new ThreadPool with a fixed number of threads
    pub fn new(size: usize, name_prefix: &str) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver), name_prefix.to_string()));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Execute a closure on the thread pool
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        if let Some(sender) = &self.sender {
            sender.send(job).expect("Thread pool sender failed");
        }
    }

    /// Get the number of workers
    pub fn size(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drop the sender to signal workers to stop
        drop(self.sender.take());

        // Wait for all workers to finish
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().ok();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>, name_prefix: String) -> Worker {
        let thread = thread::Builder::new()
            .name(format!("{}-{}", name_prefix, id))
            .spawn(move || loop {
                let job = receiver.lock().unwrap().recv();

                match job {
                    Ok(job) => {
                        job();
                    }
                    Err(_) => {
                        // Channel closed, exit
                        break;
                    }
                }
            })
            .expect("Failed to spawn worker thread");

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    #[test]
    fn test_thread_pool_basic() {
        let pool = ThreadPool::new(4, "test");
        let counter = Arc::new(AtomicU64::new(0));

        for _ in 0..10 {
            let counter = counter.clone();
            pool.execute(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }

        // Give workers time to process
        thread::sleep(Duration::from_millis(50));

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_thread_pool_size() {
        let pool = ThreadPool::new(8, "test");
        assert_eq!(pool.size(), 8);
    }
}
