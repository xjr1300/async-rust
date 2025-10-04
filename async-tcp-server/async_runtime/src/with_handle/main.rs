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

use async_std::task::sleep;
use waker_fn::waker_fn;

fn main() {
    test1();
    test2();
}

fn test1() {
    let executor = Executor::new();
    let executor_clone = Arc::clone(&executor);
    Arc::clone(&executor).spawn(async move {
        let handle_f = executor.spawn(f());
        let handle_g = executor.spawn(g());
        println!(
            "got value {}",
            handle_f.await.unwrap() + handle_g.await.unwrap()
        );
    });
    executor_clone.run();
}

async fn f() -> i64 {
    println!("in f sleeping 1");
    sleep(Duration::from_secs(1)).await;
    println!("in f calling g");
    let v = g().await;
    println!("in f sleeping 2");
    sleep(Duration::from_secs(1)).await;
    println!("in f return v");
    v
}

async fn g() -> i64 {
    println!("in g sleeping");
    sleep(Duration::from_secs(1)).await;
    println!("in g return 10");
    10
}

fn test2() {
    let executor = Executor::new();
    let executor_clone = Arc::clone(&executor);
    Arc::clone(&executor).spawn(async move {
        let handle_f = executor.spawn(async { 10 });
        let handle_g = executor.spawn(async { 10 });
        println!(
            "got value {}",
            handle_f.await.unwrap() + handle_g.await.unwrap()
        );
    });
    executor_clone.run();
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    task_id: usize,
    waker: Arc<Waker>,
}

struct Executor {
    sender: mpsc::Sender<usize>,
    receiver: mpsc::Receiver<usize>,
    counter: AtomicUsize,
    waiting: Mutex<HashMap<usize, Task>>,
    polling: Mutex<VecDeque<Task>>,
}

impl Executor {
    #[allow(clippy::arc_with_non_send_sync)]
    fn new() -> Arc<Self> {
        let (sender, receiver) = mpsc::channel();
        Arc::new(Self {
            sender,
            receiver,
            counter: AtomicUsize::new(0),
            waiting: Mutex::new(HashMap::new()),
            polling: Mutex::new(VecDeque::new()),
        })
    }

    fn spawn_rcv<F, T>(&self, future: F) -> mpsc::Receiver<T>
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

    fn spawn<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + 'static,
        T: Send + 'static,
    {
        let rx = self.spawn_rcv(future);
        JoinHandle {
            receiver: Some(rx),
            cache: Arc::new(None.into()),
        }
    }

    fn poll(&self) {
        loop {
            // pollingキューに登録されているタスクを取得
            let mut task = match self.polling.lock().unwrap().pop_front() {
                Some(task) => task,
                // タスクがなければメソッドを終了
                None => break,
            };
            let waker = Arc::clone(&task.waker);
            let context = &mut Context::from_waker(&waker);
            match task.future.as_mut().poll(context) {
                Poll::Ready(()) => {}
                Poll::Pending => {
                    self.waiting.lock().unwrap().insert(task.task_id, task);
                }
            }
        }
    }

    fn run(&self) {
        loop {
            // pollingキューにあるタスクをすべて実行
            self.poll();

            if self.polling.lock().unwrap().is_empty() && self.waiting.lock().unwrap().is_empty() {
                break;
            }

            let task_id = self.receiver.recv().unwrap();

            if let Some(task) = self.waiting.lock().unwrap().remove(&task_id) {
                self.polling.lock().unwrap().push_back(task);
            }
        }
    }
}

struct JoinHandle<T>
where
    T: 'static,
{
    receiver: Option<mpsc::Receiver<T>>,
    cache: Arc<Mutex<Option<T>>>,
}

impl<T> Future for JoinHandle<T>
where
    T: Send + 'static,
{
    type Output = Result<T, mpsc::RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        {
            // すでに結果がキャッシュされている場合はその値を返す
            let mut guard = self.cache.lock().unwrap();
            if guard.is_some() {
                let value = guard.take().unwrap();
                return Poll::Ready(Ok(value));
            }
        }

        if self.receiver.is_none() {
            // 結果を受信する受信機は、すでに受信を待機するスレッドにムーブされているため、待機を継続
            return Poll::Pending;
        }

        // 結果の受信を試行
        match self.receiver.as_ref().unwrap().try_recv() {
            // 結果を受信できたた場合は、受信した値を結果として返す
            Ok(value) => Poll::Ready(Ok(value)),
            // 受信していない場合は、結果を受信するスレッドを起動
            Err(mpsc::TryRecvError::Empty) => {
                let waker = cx.waker().clone();
                let receiver = self.receiver.take().unwrap();
                let cache = Arc::clone(&self.cache);
                std::thread::spawn(move || {
                    // 結果を受信したらキャッシュに値を設定して、ウェイカーを呼び出すことで、
                    // 再度pollを呼び出すようにする。
                    let value = receiver.recv().unwrap();
                    cache.lock().unwrap().replace(value);
                    waker.wake();
                });
                Poll::Pending
            }
            _ => Poll::Ready(Err(mpsc::RecvError)),
        }
    }
}
