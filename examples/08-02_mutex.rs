use std::sync::Arc;
use tokio::sync::Mutex;

async fn actor_replacement(state: Arc<Mutex<i64>>, value: i64) -> i64 {
    let mut state = state.lock().await;
    *state += value;
    *state
}

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    let now = tokio::time::Instant::now();

    // let max_count = 100_000_000;
    // let max_count = 50_000_000;
    let max_count = 10_000_000;
    // let max_count = 1_000_000;
    for i in 0..max_count {
        let cloned_state = Arc::clone(&state);
        let future = async move {
            let _value = actor_replacement(cloned_state, i).await;
            // let handle = tokio::spawn(async move { actor_replacement(cloned_state, i).await });
            // let _value = handle.await.unwrap();
        };
        handles.push(tokio::spawn(future));
    }

    for handle in handles {
        handle.await.unwrap();
    }
    println!("Elapsed: {:?}", now.elapsed());
}
