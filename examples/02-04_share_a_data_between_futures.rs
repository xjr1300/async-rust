use core::task::Poll;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Context,
};

use tokio::time::Duration;

#[derive(Debug)]
enum CounterType {
    Increment,
    Decrement,
}

#[derive(Debug, Default)]
struct SharedData {
    counter: i32,
}

impl SharedData {
    fn increment(&mut self) {
        self.counter += 1;
    }
    fn decrement(&mut self) {
        self.counter -= 1;
    }
}

struct CounterFuture {
    counter_type: CounterType,
    data_reference: Arc<Mutex<SharedData>>,
    count: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        std::thread::sleep(Duration::from_secs(1));

        // Mutex::try_lockメソッドによってロックの獲得を試行する。
        // Mutex::try_lockメソッドは、lockメソッドと異なりロックが得られるまで待機せず、
        // ロックの取得成功または失敗を即座に返す。
        // Future::pollメソッドは非同期でないため、tokio::sync::Mutexを利用できない。
        // したがって、std::sync::Mutexのtry_lockメソッドを使用して、スレッドの実行を
        // 停止しない。
        let mut guard = match self.data_reference.try_lock() {
            Ok(guard) => guard,
            Err(e) => {
                println!("error for {:?}: {e}", self.counter_type);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        let value = &mut *guard;

        match self.counter_type {
            CounterType::Increment => {
                value.increment();
                println!("after increment: {}", value.counter);
            }
            CounterType::Decrement => {
                value.decrement();
                println!("after decrement: {}", value.counter);
            }
        }

        std::mem::drop(guard);

        self.count += 1;
        if self.count < 3 {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

#[tokio::main]
async fn main() {
    let shared_data2 = Arc::new(Mutex::new(SharedData::default()));
    let counter1 = CounterFuture {
        counter_type: CounterType::Increment,
        data_reference: Arc::clone(&shared_data2),
        count: 0,
    };
    let counter2 = CounterFuture {
        counter_type: CounterType::Decrement,
        data_reference: shared_data2,
        count: 0,
    };
    let handle1 = tokio::task::spawn(counter1);
    let handle2 = tokio::task::spawn(counter2);
    let _ = tokio::join!(handle1, handle2);
}
