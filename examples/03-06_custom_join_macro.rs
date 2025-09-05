use async_rust::{
    futures::{CounterFuture, async_fn},
    runtime::FutureType,
    spawn_task, try_join2,
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
    // let _results: Vec<u32> = join!(t_one, t_two);
    // let _results: Vec<()> = join!(t_three, t_four);

    // let _results: Vec<std::thread::Result<u32>> = try_join!(t_one, t_two);
    // let _results: Vec<std::thread::Result<()>> = try_join!(t_three, t_four);

    let _results: Vec<std::thread::Result<u32>> = try_join2!(u32; t_one, t_two);
    let _results: Vec<std::thread::Result<()>> = try_join2!((); t_three, t_four);
}
