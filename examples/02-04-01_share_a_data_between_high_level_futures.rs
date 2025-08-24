use std::sync::Arc;

use tokio::{sync::Mutex, time::Duration};

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

async fn count(count: u32, data: Arc<Mutex<SharedData>>, counter_type: CounterType) -> u32 {
    for _ in 0..count {
        let mut data = data.lock().await;
        match counter_type {
            CounterType::Increment => {
                data.increment();
                println!("after increment {}", data.counter);
            }
            CounterType::Decrement => {
                data.decrement();
                println!("after decrement {}", data.counter);
            }
        }
        std::mem::drop(data);
        std::thread::sleep(Duration::from_secs(1));
    }
    count
}

#[tokio::main]
async fn main() {
    let shared_data1 = Arc::new(Mutex::new(SharedData::default()));
    let shared_data2 = Arc::clone(&shared_data1);

    let handle1 =
        tokio::task::spawn(async move { count(3, shared_data1, CounterType::Increment).await });
    let handle2 =
        tokio::task::spawn(async move { count(3, shared_data2, CounterType::Decrement).await });
    let _ = tokio::join!(handle1, handle2);
}
