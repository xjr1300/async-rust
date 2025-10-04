use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, mpsc},
    task::{Context, Poll, Waker},
};

use crate::waker::create_raw_waker;

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    waker: Arc<Waker>,
}

#[derive(Default)]
pub struct Executor {
    pub polling: VecDeque<Task>,
}

impl Executor {
    pub fn spawn<F, T>(&mut self, future: F) -> mpsc::Receiver<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let future: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(async move {
            let result = future.await;
            let _ = tx.send(result);
            // Futureは`()`を返す。
            // Futureの結果はmpscチャネルの受信側から受け取る。
        });
        let task = Task {
            future,
            waker: self.create_waker(),
        };
        self.polling.push_back(task);
        rx
    }

    pub fn poll(&mut self) {
        let mut task = match self.polling.pop_front() {
            Some(task) => task,
            None => return,
        };
        let waker = task.waker.clone();
        let context = &mut Context::from_waker(&waker);
        match task.future.as_mut().poll(context) {
            Poll::Ready(()) => {}
            Poll::Pending => {
                self.polling.push_back(task);
            }
        }
    }

    pub fn create_waker(&self) -> Arc<Waker> {
        Arc::new(unsafe { Waker::from_raw(create_raw_waker()) })
    }
}
