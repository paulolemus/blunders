//! Functionality related to multi-threading.

use std::process;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// PoisonPill is used to cause the process to abort if there are
/// any panics in any thread. This may lead to a resource leak,
/// but also allows us to better handle bugs in threads.
/// TODO: Remove after squashing bugs.
pub struct PoisonPill;

impl Drop for PoisonPill {
    fn drop(&mut self) {
        if thread::panicking() {
            process::exit(1);
        }
    }
}

/// Type of function accepted as a runnable job for a Thread.
type Job = Box<dyn FnOnce() + Send + 'static>;

/// Message passed from ThreadPool to Threads to give jobs or signal termination.
enum Message {
    NewJob(Job),
    Terminate,
}

/// Long lived Thread type. Each Thread receives commands through a receiver.
#[derive(Debug)]
struct Thread {
    pub _id: usize,    // TODO
    pub _name: String, // TODO
    handle: Option<JoinHandle<()>>,
}

impl Thread {
    /// Spawn a new thread
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Self {
        let runner = move || {
            // Shutdown process on any panics.
            let _poison = PoisonPill;

            loop {
                let recv_result = { receiver.lock().unwrap().recv() };

                match recv_result {
                    Ok(message) => match message {
                        Message::NewJob(job) => {
                            job();
                        }
                        Message::Terminate => break,
                    },

                    // Sender has closed, allow thread graceful exit.
                    Err(_) => break,
                }
            }
        };

        let name = format!("Thread {id}");
        let handle = thread::Builder::new()
            .name(name.clone())
            .spawn(runner)
            .unwrap();

        Self {
            _id: id,
            _name: name,
            handle: Some(handle),
        }
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        let handle_opt = self.handle.take();
        if let Some(handle) = handle_opt {
            let _ = handle.join();
        }
    }
}

/// Long-lived thread pool containing n threads for job processing.
///
/// Requirements:
/// ThreadPool needs to know which threads are available at any given time.
/// A ThreadPool is expected to live for the duration of the engine.
/// Must be sharable b/t threads.
/// The ThreadPool manages all threads within it, the threads may not outlive it.
#[derive(Debug)]
pub struct ThreadPool {
    num_threads: usize,
    _threads: Vec<Thread>, // TODO
    sender: Sender<Message>,
    receiver: Arc<Mutex<Receiver<Message>>>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `num_threads` persistent worker threads.
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut threads = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            threads.push(Thread::new(id, Arc::clone(&receiver)));
        }

        Self {
            num_threads,
            _threads: threads,
            sender,
            receiver,
        }
    }

    /// Send a runnable job to an available Thread in the ThreadPool to run.
    pub fn run<J: Into<Job>>(&self, job: J) {
        self.sender.send(Message::NewJob(job.into())).unwrap()
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Clear all pending jobs in queue.
        {
            let locked_receiver = self.receiver.lock().unwrap();
            while locked_receiver.try_recv().is_ok() {}
        }

        // Tell each thread to terminate.
        for _ in 0..self.num_threads {
            let _ = self.sender.send(Message::Terminate);
        }
    }
}
