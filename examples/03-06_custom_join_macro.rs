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
use flume::{Receiver, Sender};
use futures_lite::future;

#[derive(Debug, Clone, Copy)]
enum FutureType {
    High,
    Low,
}

/// カウンターを持つFuture
///
/// pollが呼ばれるたびにカウンターをインクリメントし、カウンターが3になったら完了する。
struct CounterFuture {
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.count += 1;
        println!("polling with result: {}", self.count);

        // 本来、poll内でブロッキング処理をするのは非推奨
        std::thread::sleep(Duration::from_secs(1));

        if self.count < 3 {
            // wakerを呼び出して、再度pollされるように登録
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

fn spawn_task<F, T>(future: F, order: FutureType) -> Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // 優先度が高いタスクキュー
    static HIGH_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> =
        LazyLock::new(flume::unbounded::<Runnable>);
    // 優先度が低いタスクキュー
    static LOW_CHANNEL: LazyLock<(Sender<Runnable>, Receiver<Runnable>)> =
        LazyLock::new(flume::unbounded::<Runnable>);

    // 優先度が高いキューを優先的に処理するスレッドを2つ起動し、HIGH_CHANNELへのSenderを返す
    static HIGH_QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        for _ in 0..2 {
            let high_receiver = HIGH_CHANNEL.1.clone();
            let low_receiver = LOW_CHANNEL.1.clone();
            thread::spawn(move || {
                loop {
                    match high_receiver.try_recv() {
                        Ok(runnable) => {
                            let _ = catch_unwind(|| runnable.run());
                        }
                        Err(_) => match low_receiver.try_recv() {
                            Ok(runnable) => {
                                let _ = catch_unwind(|| runnable.run());
                            }
                            Err(_) => {
                                thread::sleep(Duration::from_millis(100));
                            }
                        },
                    }
                }
            });
        }
        HIGH_CHANNEL.0.clone()
    });

    // 優先度が低いキューを処理するスレッドを起動し、LOW_CHANNELのSenderを返す
    static LOW_QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        let low_receiver = LOW_CHANNEL.1.clone();
        let high_receiver = HIGH_CHANNEL.1.clone();
        thread::spawn(move || {
            loop {
                match low_receiver.try_recv() {
                    Ok(runnable) => {
                        let _ = catch_unwind(|| runnable.run());
                    }
                    Err(_) => match high_receiver.try_recv() {
                        Ok(runnable) => {
                            let _ = catch_unwind(|| runnable.run());
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                    },
                }
            }
        });
        LOW_CHANNEL.0.clone()
    });

    // 優先度が高いキューにタスクを送信するクロージャー
    let schedule_high = |runnable| HIGH_QUEUE.send(runnable).unwrap();
    // 優先度が低いキューにタスクを送信するクロージャー
    let schedule_low = |runnable| LOW_QUEUE.send(runnable).unwrap();

    // 引数で渡された優先度によってスケジューラを切り替える
    let schedule = match order {
        FutureType::High => schedule_high,
        FutureType::Low => schedule_low,
    };
    // タスクを生成
    let (runnable, task) = async_task::spawn(future, schedule);
    // タスクをスケジューリング（キューに投入）
    runnable.schedule();
    // block_onで待ち合わせ可能なタスクハンドルを返す
    task
}

async fn async_fn() {
    std::thread::sleep(Duration::from_secs(1));
    println!("async fn");
}

/// spawn_taskを呼び出すマクロ
///
/// 優先度を指定しない場合は、FutureType::Lowに設定
macro_rules! spawn_task {
    ($future:expr) => {
        spawn_task!($future, FutureType::Low)
    };
    ($future:expr, $order:expr) => {
        spawn_task($future, $order)
    };
}

#[allow(unused_macros)]
macro_rules! join {
    ($($future:expr),*) => {
        {
            vec![$(future::block_on($future)),*]
        }
    }
}

#[allow(unused_macros)]
macro_rules! try_join {
    ($($future:expr),*) => {
        {
            let mut results = vec![];
            $(
                let result = catch_unwind(|| future::block_on($future));
                results.push(result);
            )*
            results
        }
    }
}

#[allow(unused_macros)]
macro_rules! try_join2 {
    ($t:ty; $($future:expr),* $(,)?) => {
        {
            let mut results: Vec<std::thread::Result<$t>> = vec![];
            $(
                let result = catch_unwind(|| future::block_on($future));
                results.push(result);
            )*
            results
        }
    }
}

fn main() {
    let one = CounterFuture { count: 0 };
    let two = CounterFuture { count: 0 };

    // 優先度が高いキューに送信するタスク
    let t_one = spawn_task!(one, FutureType::High);
    // 優先度が低いキューに送信するタスク
    let t_two = spawn_task!(two);
    // 優先度が低いキューに送信するタスク
    let t_three = spawn_task!(async_fn());
    // 優先度が高いキューに送信するタスク
    let t_four = spawn_task!(
        async {
            async_fn().await;
            async_fn().await;
        },
        FutureType::High
    );

    // タスクの完了を待つ
    // let _results: Vec<u32> = join!(t_one, t_two);
    // let _results: Vec<()> = join!(t_three, t_four);

    // let _results: Vec<std::thread::Result<u32>> = try_join!(t_one, t_two);
    // let _results: Vec<std::thread::Result<()>> = try_join!(t_three, t_four);

    let _results: Vec<std::thread::Result<u32>> = try_join2!(u32; t_one, t_two);
    let _results: Vec<std::thread::Result<()>> = try_join2!((); t_three, t_four);
}
