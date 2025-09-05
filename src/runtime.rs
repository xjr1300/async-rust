use std::{panic::catch_unwind, sync::LazyLock, time::Duration};

use async_task::{Runnable, Task};
use flume::{Receiver, Sender};

use crate::{join, spawn_task};

#[derive(Debug, Clone, Copy)]
pub enum FutureType {
    High,
    Low,
}

pub fn spawn_task_function<F, T>(future: F, order: FutureType) -> Task<T>
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
        let high_num = std::env::var("HIGH_NUM").unwrap().parse::<usize>().unwrap();
        for _ in 0..high_num {
            let high_receiver = HIGH_CHANNEL.1.clone();
            let low_receiver = LOW_CHANNEL.1.clone();
            std::thread::spawn(move || {
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
                                std::thread::sleep(Duration::from_millis(100));
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
        let low_num = std::env::var("LOW_NUM").unwrap().parse::<usize>().unwrap();
        for _ in 0..low_num {
            let low_receiver = LOW_CHANNEL.1.clone();
            let high_receiver = HIGH_CHANNEL.1.clone();
            std::thread::spawn(move || {
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
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        },
                    }
                }
            });
        }
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
    // タスクをスケジューリング（エグゼキューターのキューに投入）
    runnable.schedule();
    // block_onで待ち合わせ可能なタスクハンドルを返す
    task
}

pub struct Runtime {
    high_num: usize,
    low_num: usize,
}

impl Runtime {
    #[allow(clippy::new_without_default)]
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
