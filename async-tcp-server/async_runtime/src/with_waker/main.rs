use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use waker_fn::waker_fn;

fn main() {
    let executor = Executor::new();
    let rx = executor.spawn_rcv(async {
        async_std::task::sleep(Duration::from_secs(1)).await;
        10
    });
    executor.run();
    println!("got value {:}", rx.recv().unwrap());
}

pub struct Task {
    /// タスクが処理するフューチャー
    future: Pin<Box<dyn Future<Output = ()>>>,
    /// タスクID
    ///
    /// エグゼキューターが採番
    task_id: usize,
    /// エグゼキューターに再度このタスクをポーリングさせるためのウェイカー
    waker: Arc<Waker>,
}

pub struct Executor {
    sender: mpsc::Sender<usize>,
    receiver: mpsc::Receiver<usize>,
    /// 非同期処理が終了せず、その非同期処理の終了を待っているタスクに採番するカウンター
    counter: AtomicUsize,
    /// 待機しているタスクについて、タスクIDとそのタスクIDを割り当てられたタスクを格納するハッシュマップ
    waiting: Mutex<HashMap<usize, Task>>,
    /// タスクの準備ができて、ポーリングされることを待っているタスクを格納するキュー（デック）
    polling: Mutex<VecDeque<Task>>,
}

impl Executor {
    pub fn new() -> Arc<Self> {
        let (sender, receiver) = mpsc::channel();
        #[allow(clippy::arc_with_non_send_sync)]
        Arc::new(Executor {
            sender,
            receiver,
            counter: AtomicUsize::new(0),
            waiting: Mutex::new(HashMap::new()),
            polling: Mutex::new(VecDeque::new()),
        })
    }

    pub fn spawn_rcv<F, T>(&self, future: F) -> mpsc::Receiver<T>
    where
        F: Future<Output = T> + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let future: Pin<Box<dyn Future<Output = ()>>> = Box::pin(async move {
            let result = future.await;
            let _ = tx.send(result);
        });

        let task_id = self.counter.fetch_add(1, Ordering::SeqCst);
        let sender = self.sender.clone();
        let task = Task {
            future,
            task_id,
            waker: waker_fn(move || sender.send(task_id).expect("send")).into(),
        };
        self.polling.lock().unwrap().push_back(task);
        rx
    }

    /// pollingキューに登録されているタスクをすべて実行する。
    pub fn poll(&self) {
        loop {
            // pollingキューに登録されているタスクをキューから取り出す。
            // タスクを取り出せなかった場合（Noneの場合）、ループを終了
            let mut task = match self.polling.lock().unwrap().pop_front() {
                Some(task) => task,
                None => break,
            };
            let waker: Arc<Waker> = Arc::clone(&task.waker);
            let context = &mut Context::from_waker(&waker);
            match task.future.as_mut().poll(context) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    // Pendingな場合はwaitingに登録
                    self.waiting.lock().unwrap().insert(task.task_id, task);
                }
            }
        }
    }

    pub fn run(&self) {
        loop {
            // pollingキューにあるタスクをすべて実行
            self.poll();

            // 実行するタスクと待機しているタスクがない場合はエグゼキューターを終了
            if self.polling.lock().unwrap().is_empty() && self.waiting.lock().unwrap().is_empty() {
                break;
            }

            // ウェイカーからの通知を待機
            let task_id = self.receiver.recv().unwrap();

            // ウェイカーから受け取ったタスクIDに対応するタスクを、pollingキューに登録
            if let Some(task) = self.waiting.lock().unwrap().remove(&task_id) {
                self.polling.lock().unwrap().push_back(task);
            }
        }
    }
}
