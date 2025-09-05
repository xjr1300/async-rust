use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use async_rust::{
    futures::{CounterFuture, async_fn},
    join,
    runtime::{FutureType, Runtime},
    spawn_task,
};

struct BackgroundProcess;

impl Future for BackgroundProcess {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("background process firing");
        std::thread::sleep(Duration::from_secs(1));
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn main() {
    Runtime::new().with_low_num(2).with_high_num(4).run();

    // 次のコードは、_backgroundがスコープ外になると、タスクがドロップされて、タスクが実行されない
    // 可能性がある。
    // let _background = spawn_task!(BackgroundProcess {});
    // したがって、`detach`メソッドを使用して、バックグラウンドスレッドの実行を続けさせる。

    spawn_task!(BackgroundProcess {}).detach();
    let one = CounterFuture::new(0, 3);
    let two = CounterFuture::new(0, 3);

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

    let _outcome: Vec<u32> = join!(t_one, t_two);
    let _outcome_two: Vec<()> = join!(t_four, t_three);
}
