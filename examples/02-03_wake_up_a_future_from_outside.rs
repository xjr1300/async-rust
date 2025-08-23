use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use tokio::sync::mpsc;
use tokio::task;

struct MyFuture {
    state: Arc<Mutex<MyFutureState>>,
}

#[derive(Debug, Default)]
struct MyFutureState {
    data: Option<Vec<u8>>,
    waker: Option<Waker>,
}

impl MyFuture {
    fn new() -> (Self, Arc<Mutex<MyFutureState>>) {
        let state = Arc::new(Mutex::new(MyFutureState::default()));
        (
            Self {
                state: state.clone(),
            },
            state,
        )
    }
}

impl Future for MyFuture {
    type Output = String;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("Polling the future");

        let mut state = self.state.lock().unwrap();

        if state.data.is_some() {
            let data = state.data.take().unwrap();
            println!("Return Poll::Ready from the future");
            Poll::Ready(String::from_utf8(data).unwrap())
        } else {
            state.waker = Some(cx.waker().clone());
            println!("Return Poll::Pending from the future");
            Poll::Pending
        }
    }
}

#[tokio::main]
async fn main() {
    let (my_future, state) = MyFuture::new();
    let (tx, mut rx) = mpsc::channel::<()>(1);
    let task_handle = task::spawn(my_future);
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    println!("Spawning trigger task");

    let trigger_task = task::spawn(async move {
        // 受信機は、送信機から通知を受け取るまで待機
        rx.recv().await;

        // 通知を受け取ったら共有した状態にデータを設定
        let mut state = state.lock().unwrap();
        state.data = Some(b"Hello from the outside".to_vec());

        // エクゼキューターがタスクを起こす(Futureのpollメソッドを呼び出す）ように、
        // エクゼキューターを起こす。
        // 共有状態のウェイカーは、エクゼキューターがFutureのpollメソッドを呼び出し、
        // pollメソッドがPoll::Pendingを返す場合に、pollメソッドで設定されるまで
        // 無限ループで待機する。
        // 共有状態のウェイカーを得られた場合、エクゼキューターを直接起こす。
        loop {
            if let Some(waker) = state.waker.take() {
                waker.wake();
                break;
            }
        }
    });

    // 受信機が通知を送信
    tx.send(()).await.unwrap();

    // task_handleは、共有状態にデータが設定されるまで待機
    let outcome = task_handle.await.unwrap();
    println!("Task completed with outcome: {outcome}");

    // trigger_taskは次を待機
    // 1. 送信機からの通知
    // 2. 共有状態のロック
    // 3. 共有状態にウェイカーが設定される
    // 共有状態にウェイカーが設定されるのは、エグゼキューターがFutureのpollメソッドの初回呼び出しで、
    // pollメソッドがPoll::Pendingを返すときに設定される。
    trigger_task.await.unwrap();
}
