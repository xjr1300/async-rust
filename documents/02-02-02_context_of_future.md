# Future

1. あるフューチャーのpoll関数が呼ばれたとき、そのフューチャが値を返すためには、別の非同期操作を実行して、その完了を待つ必要がある。
2. そのフューチャーは、その非同期操作をウェイカーを含んだコンテクストを引数として呼び出す。これで、その非同期操作が終了したら、再度pollが呼ばれるように登録したことになる。
3. 非同期操作を行うルーチンは、非同期操作とウェイカーとの対応をキューで管理する。
4. 時間が経過して、非同期操作が終了すると、その通知がくる。非同期操作を管理するルーチンは、対応するウェイカーをキューから取り出し、それぞれに対してwake_by_refを呼び出して、フューチャーを呼び起こす。
5. wake_by_ref関数が呼ばれると、対応するタスクがスケジュールの対象になる。この方法はランタイムによって異なる。
6. エグゼキューターは、再度フューチャーのpollメソッドを呼び出す。フューチャーは問題の非同期操作が終了していることを確認して、終了していたら値を返す。

Rustの`Future`は「まだ結果が出ていない計算を表す値」である。
それが実際に実行される時には、`poll`メソッドが呼ばれて進捗が確認される。

## 1. pollが呼ばれる

Tokioなどのエグゼキューターがタスクを実行する際、`Future`の`poll`が呼ばれる。
`poll`が結果を返せる場合は`Poll::Ready(T)`を返し、結果を返せない場合は`Poll::Pending`を返す。

## 2. Poll::Pendingのときの処理

フューチャー内部で「別の非同期操作」（例： I/Oやタイマー）を開始する必要がある。
そのとき、`poll`の引数で受け取った`Context`から`Waker`を取得して、`wake_by_ref`で非同期操作の管理ルーチンに登録する。
これは、非同期操作が終了したら、この`Waker`を呼び出すように要請している。

## 3. ランタイムの非同期操作管理

非同期操作はOSからの通知を待つ。
その管理ルーチンは、どの操作がどの`Waker`と結びついているかをキューで管理している。

## 4. 操作が終わるとWakerが呼ばれる

OSから「読み込み完了」などの通知がくると、ランタイムは対応する`Waker`を`wake_by_ref`で呼び出す。
これで、「この`Future`を`poll`するべき」であることをエグゼキューターに伝わる。

## 5. タスクの再スケジューリング

`wake_by_ref`によって、そのタスクは実行キューに入る。
エグゼキューターが再びそのタスクを取り出し、`poll`を呼び出す。

## 6. 結果が返る

次に`poll`をよびだしたとき、非同期操作は終了している。
したがって、`Poll::Ready<T>`を返して非同期操作が終了する。

## TimerFuture

```rust
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

/// 指定した時間が経過すると完了するタイマーフューチャー
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
    // 最初は、同期処理が完了していないため、Poll::Pendingを返す。
    // 別スレッドが2秒間スリープた後、完了フラグを立ててWakerを起こす。
    // 再度poll関数が呼び出されてPoll::Readyを返す。
    block_on(future);
    println!("Timer done!");
}
```
