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

/// 非同期タスクをspawnする関数
///
/// 引数のFutureを実行するTaskを生成し、Taskハンドルを返す。
///
/// 戻り値のTaskをblock_onすることで、最終的な出力を得られる。
fn spawn_task<F, T>(future: F) -> Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    // グローバルなタスクキュー（送信側）で、最初のアクセス時に初期化
    static QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
        //
        // このブロックはQUEUEに最初にアクセスされた時のみ実行される。
        // 2回目以降のアクセスでは、この初期化ブロックは実行されない。
        //
        // マルチプロデューサ・マルチコンシューマーチャネルを作成
        let (tx, rx) = flume::unbounded::<Runnable>();

        // タスクを受信して実行するスレッドを起動
        //
        // - タスクを受信したらRunnableのrunメソッドを呼び出す
        // - runメソッドはフューチャーを一度だけpollする
        // - Poll::PendingならWakerが登録され、後で再スケジュールされる
        // - Poll::Readyならタスクは完了する
        thread::spawn(move || {
            // Runnableを受け取るまでブロックするループ
            while let Ok(runnable) = rx.recv() {
                println!("runnable accepted");
                // Runnable実行中にパニックが発生した場合、catch_unwindでキャッチする。
                // パニックをキャッチすることで、この簡易ランタイムが異常停止しないようにする。
                //
                // Runnable::runメソッド呼び出し後、タスクが未完了であれば
                // Wakerを介してRunnableが再びスケジュールされる。
                // そのため、Runnableは必要に応じて再実行され続ける。
                let _ = catch_unwind(|| runnable.run());
            }
        });

        println!("QUEUE was initialized");

        // プロデューサー側を返す
        tx
    });

    // Runnableをキューに送信するクロージャー（スケジューラ関数）を定義
    let schedule = |runnable| QUEUE.send(runnable).unwrap();

    // 同じフューチャーを共有するRunnableとTaskを生成する。
    // - Runnable: タスクをpollするための実行可能オブジェクト
    // - Task: ユーザーが最終的に.awaitやblock_onで待つためのハンドル
    //
    // Runnable::runはフューチャーを一度pollする。
    // Poll::PendingならWaker経由で再度Runnableがスケジュールされる。
    // これによりタスクは何度もpollされ、最終的にPoll::Readyで完了する。
    let (runnable, task) = async_task::spawn(future, schedule);

    // Runnableをスケジュールして、タスクを処理キューに配置
    runnable.schedule();
    println!("Here is the queue count: {}", QUEUE.len());
    task
}

/// 単純なカスタムFuture
/// pollが呼ばれるたびにカウントアップし、3回目で完了する。
struct CounterFuture {
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.count += 1;
        println!("polling with result: {}", self.count);

        // 実際の非同期処理では使わないが、ここでは挙動確認のため
        // 明示的にスレッドをブロックする。
        std::thread::sleep(Duration::from_secs(1));

        if self.count < 3 {
            // 未完了なので、再度pollしてもらうためにWakerを通知
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            // 完了したため最終結果を返す
            Poll::Ready(self.count)
        }
    }
}

// std::thread::sleep関数を呼び出しているため、このタスクの実行中は
// スレッド全体がブロックされ、他のタスクも実行されなくなる。
// 非同期に待機するにはtokio::time::sleepなどの非同期版をawaitする必要がある。
async fn async_fn() {
    std::thread::sleep(Duration::from_secs(1));
    println!("async_fn");
}

fn main() {
    let one = CounterFuture { count: 0 };
    let two = CounterFuture { count: 0 };

    // ここの時点では、フューチャーは作成されたが、エグゼキューターにスケジュールされていないため、
    // まだポーリングは始まらない。
    // したがって、フューチャーは進んでいない。
    println!("CounterFutures were created");

    let t_one = spawn_task(one);
    println!("Task one was spawned");

    let t_two = spawn_task(two);
    println!("Task two was spawned");

    // asyncブロックをspawnして複数回async_fnを実行する。
    let t_three = spawn_task(async {
        async_fn().await;
        async_fn().await;
        async_fn().await;
        async_fn().await;
    });
    println!("Task three was spawned");

    // メインスレッドを強制的にブロック
    // （ここでも他のタスクは進行しないことに注意）
    std::thread::sleep(Duration::from_secs(5));
    println!("sleeping of main thread is over");

    println!("before the block_on");

    // Taskハンドルをblock_onすると、最終的な結果が得られる。
    // block_onが完了するのは、タスクがPoll::Readyを返したとき。
    future::block_on(t_one);
    future::block_on(t_two);
    future::block_on(t_three);

    println!("after the block_on");
}
