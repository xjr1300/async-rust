use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use futures::executor::block_on;

/// 内部状態を保持する構造体
struct SharedState {
    completed: bool,      // タイマーが終わったかどうか
    waker: Option<Waker>, // 起こすべきタスク
}

/// 一定時間後に完了する Future
pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

impl TimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // 別スレッドで時間経過を待つ
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let mut state = thread_shared_state.lock().unwrap();
            state.completed = true;
            if let Some(waker) = state.waker.take() {
                // 完了したので、登録されたタスクを起こす
                waker.wake();
            }
        });

        TimerFuture { shared_state }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();

        if state.completed {
            Poll::Ready(())
        } else {
            // まだ終わっていなければ Waker を登録して Pending を返す
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn main() {
    let future = TimerFuture::new(Duration::from_secs(2));
    println!("Start 2秒待ち…");
    // block_onによってpollが呼び出される。
    // 未完了なためWakerを登録してPoll::Pendingを返す。
    block_on(future);
    // この間に、別スレッドでスリープして2秒後に完了フラグを立ててWakerを起こす。
    // poll関数が呼び出されてPoll::Readyを返す。
    println!("Timer done!");
}
