//! 見出しは「ワーカ数とキューを増やす」となっているが、ワーカーの数は増えているが、キューの数は増えていない。
//! 実際には、キューを監視するワーカが増えているだけである。
use std::{
    future::Future,
    panic::catch_unwind,
    pin::Pin,
    sync::LazyLock,
    task::{Context, Poll},
    thread,
    time::Duration,
};

use async_task::{Runnable, Task};
use futures_lite::future;

fn spawn_task<F, T>(future: F) -> Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    static QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        let (tx, rx) = flume::unbounded::<Runnable>();
        // cloneしているため、**キューは増えていない**。
        let queue_one = rx.clone();
        let queue_two = rx;
        thread::spawn(move || {
            while let Ok(runnable) = queue_one.recv() {
                println!("one: runnable accepted");
                let _ = catch_unwind(|| runnable.run());
            }
        });
        thread::spawn(move || {
            while let Ok(runnable) = queue_two.recv() {
                println!("two: runnable accepted");
                let _ = catch_unwind(|| runnable.run());
            }
        });
        tx
    });

    let schedule = |runnable| QUEUE.send(runnable).unwrap();
    let (runnable, task) = async_task::spawn(future, schedule);
    runnable.schedule();
    println!("Here is the queue count: {}", QUEUE.len());
    task
}

struct CounterFuture {
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.count += 1;
        println!("polling with result: {}", self.count);

        std::thread::sleep(Duration::from_secs(1));

        if self.count < 3 {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

async fn async_fn() {
    std::thread::sleep(Duration::from_secs(1));
    println!("async_fn");
}

fn main() {
    let one = CounterFuture { count: 0 };
    let two = CounterFuture { count: 0 };

    let t_one = spawn_task(one);
    let t_two = spawn_task(two);
    let t_three = spawn_task(async {
        async_fn().await;
        async_fn().await;
        async_fn().await;
        async_fn().await;
    });
    println!("Task three was spawned");

    std::thread::sleep(Duration::from_secs(5));

    future::block_on(t_one);
    future::block_on(t_two);
    future::block_on(t_three);
}
