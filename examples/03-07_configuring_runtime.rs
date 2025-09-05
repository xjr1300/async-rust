use async_rust::{
    futures::{CounterFuture, async_fn},
    join,
    runtime::{FutureType, Runtime},
    spawn_task,
};

fn main() {
    Runtime::new().with_low_num(2).with_high_num(4).run();
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

    let _: Vec<u32> = join!(t_one, t_two);
    let _: Vec<()> = join!(t_four, t_three);
}
