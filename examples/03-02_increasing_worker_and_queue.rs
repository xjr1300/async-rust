//! 見出しは「ワーカ数とキューを増やす」となっているが、ワーカーの数は増えているが、キューの数は増えていない。
//! 実際には、キューを監視するワーカが増えているだけである。
use std::{future::Future, panic::catch_unwind, sync::LazyLock, thread, time::Duration};

use async_task::{Runnable, Task};
use futures_lite::future;

use async_rust::futures::{CounterFuture, async_fn};

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

fn main() {
    let one = CounterFuture::new(0, 3);
    let two = CounterFuture::new(0, 3);

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
