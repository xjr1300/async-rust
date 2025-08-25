use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    task::Poll,
    time::Duration,
};

use rand::Rng as _;

struct ProduceNumber {
    queue: Arc<Mutex<VecDeque<i32>>>,
    count: usize,
}

impl Future for ProduceNumber {
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if self.count >= 10 {
            return Poll::Ready(true);
        }

        let mut queue = self.queue.lock().unwrap();
        let mut rng = rand::rng();
        let value = rng.random_range(0..100);
        queue.push_back(value);
        println!(
            "pushed value: {value}, number of entries after pushed: {}",
            queue.len()
        );

        // MutexGuardをドロップすることで、自身の不変参照をドロップ
        std::mem::drop(queue);

        std::thread::sleep(Duration::from_secs(1));

        self.count += 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

struct ConsumeNumber {
    queue: Arc<Mutex<VecDeque<i32>>>,
    count: usize,
}

impl Future for ConsumeNumber {
    type Output = bool;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() && self.count >= 10 {
            return Poll::Ready(true);
        }
        let value = queue.pop_front();
        if value.is_none() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let value = value.unwrap();
        println!(
            "popped value: {value}, number of entries after popped: {}",
            queue.len()
        );

        std::mem::drop(queue);

        std::thread::sleep(Duration::from_secs(1));

        self.count += 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let queue = Arc::new(Mutex::new(VecDeque::default()));
    let cloned_queue = Arc::clone(&queue);

    let producer = ProduceNumber {
        queue: cloned_queue,
        count: 0,
    };
    let producer_handle = tokio::task::spawn(producer);

    let consumer = ConsumeNumber { queue, count: 0 };
    let consumer_handle = tokio::task::spawn(consumer);
    let _ = tokio::join!(producer_handle, consumer_handle);
}
