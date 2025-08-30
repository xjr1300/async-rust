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

macro_rules! join {
    ($($future:expr),*) => {
        {
            vec![$(future::block_on($future)),*]
        }
    }
}

struct Runtime {
    /// 高優先度キューの消費スレッド数
    high_num: usize,
    /// 低優先度キューの消費スレッド数
    low_num: usize,
}

impl Runtime {
    pub fn new() -> Self {
        let num_cores = std::thread::available_parallelism().unwrap().get();
        Self {
            high_num: num_cores - 2,
            low_num: 1,
        }
    }

    pub fn with_high_num(mut self, num: usize) -> Self {
        self.high_num = num;
        self
    }

    pub fn with_low_num(mut self, num: usize) -> Self {
        self.low_num = num;
        self
    }

    pub fn run(&self) {
        unsafe {
            std::env::set_var("HIGH_NUM", self.high_num.to_string());
            std::env::set_var("LOW_NUM", self.low_num.to_string());
        }
        let high = spawn_task!(async {}, FutureType::High);
        let low = spawn_task!(async {}, FutureType::Low);
        join!(high, low);
    }
}

struct CounterFuture {
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
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
    println!("async fn");
}

fn main() {
    Runtime::new().with_low_num(2).with_high_num(4).run();
    let one = CounterFuture { count: 0 };
    let two = CounterFuture { count: 0 };

    let t_one = spawn_task!(one, FutureType::High);
    let t_two = spawn_task!(two);
    let t_three = spawn_task!(async_fn());
    let t_four = spawn_task!(
        async {
            async_fn().await;
            async_fn().await;
        },
        FutureType::High
    );

    let _: Vec<u32> = join!(t_one, t_two);
    let _: Vec<()> = join!(t_four, t_three);
}
