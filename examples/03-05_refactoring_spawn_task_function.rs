use futures_lite::future;

use async_rust::{
    futures::{CounterFuture, async_fn},
    runtime::FutureType,
    spawn_task,
};

fn main() {
    unsafe {
        std::env::set_var("HIGH_NUM", "4");
        std::env::set_var("LOW_NUM", "1");
    }

    let one = CounterFuture::new(0, 3);
    let two = CounterFuture::new(0, 3);

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
    future::block_on(t_one);
    future::block_on(t_two);
    future::block_on(t_three);
    future::block_on(t_four);
}
