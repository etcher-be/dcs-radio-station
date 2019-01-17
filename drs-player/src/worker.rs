use std::sync::mpsc::{channel, Receiver, RecvError, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Worker<T> {
    tx: Sender<Command>,
    join_handle: JoinHandle<T>,
}

pub struct Context {
    rx: Receiver<Command>,
}

enum Command {
    Stop,
    Pause,
    Unpause,
}

impl<T> Worker<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(Context) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = channel();
        Worker {
            tx,
            join_handle: thread::spawn(|| f(Context { rx })),
        }
    }

    pub fn stop(self) {
        if let Err(_) = self.tx.send(Command::Stop) {
            error!("Error sending stop signal to worker thread");
        }
        self.join();
    }

    pub fn pause(&self) {
        if let Err(_) = self.tx.send(Command::Pause) {
            error!("Error sending pause signal to worker thread");
        }
    }

    pub fn unpause(&self) {
        if let Err(_) = self.tx.send(Command::Unpause) {
            error!("Error sending unpause signal to worker thread");
        }
    }

    pub fn join(self) {
        if let Err(_) = self.join_handle.join() {
            error!("Error joining worker thread");
        }
    }
}

impl Context {
    pub fn should_stop(&self) -> bool {
        match self.rx.try_recv() {
            Ok(Command::Pause) => {
                return self.pause_handler();
            }
            Ok(Command::Unpause) | Err(TryRecvError::Empty) => false,
            Ok(Command::Stop) | Err(TryRecvError::Disconnected) => true,
        }
    }

    pub fn should_stop_timeout(&self, timeout: Duration) -> bool {
        match self.rx.recv_timeout(timeout) {
            Ok(Command::Pause) => {
                return self.pause_handler();
            }
            Ok(Command::Unpause) | Err(RecvTimeoutError::Timeout) => false,
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => true,
        }
    }

    fn pause_handler(&self) -> bool {
        loop {
            match self.rx.recv() {
                Ok(Command::Unpause) => break,
                Ok(Command::Pause) => continue,
                Ok(Command::Stop) | Err(RecvError) => {
                    return true;
                }
            }
        }
        false
    }
}
