use tokio::task::JoinHandle;

use async_rust::futures::CounterFuture;

#[allow(clippy::redundant_async_block)]
#[tokio::main]
async fn main() {
    let counter_one = CounterFuture::new(0, 5);
    let counter_two = CounterFuture::new(0, 5);
    let handle_one: JoinHandle<u32> = tokio::task::spawn(async move { counter_one.await });
    let handle_two: JoinHandle<u32> = tokio::task::spawn(async move { counter_two.await });
    let _ = tokio::join!(handle_one, handle_two);
}
