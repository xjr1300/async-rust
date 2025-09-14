use std::{cell::UnsafeCell, collections::HashMap, sync::LazyLock};

use tokio::signal::unix::{SignalKind, signal};
use tokio_util::task::LocalPoolHandle;

#[allow(dead_code)]
async fn cleanup() {
    println!("cleanup background task started");
    let mut count = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        tokio::signal::ctrl_c().await.unwrap();
        println!("ctrl-c received!");
        count += 1;
        if count > 2 {
            std::process::exit(0);
        }
    }
}

static RUNTIME: LazyLock<LocalPoolHandle> = LazyLock::new(|| LocalPoolHandle::new(4));

thread_local! {
    static COUNTER: UnsafeCell<HashMap<u32, u32>> = UnsafeCell::new(HashMap::new());
}

fn extract_data_from_thread() -> HashMap<u32, u32> {
    let mut extracted_counter = HashMap::new();
    COUNTER.with(|counter| {
        let counter = unsafe { &mut *counter.get() };
        extracted_counter = counter.clone();
    });
    extracted_counter
}

async fn get_complete_count() -> HashMap<u32, u32> {
    let mut complete_counter = HashMap::new();
    let mut extracted_counters = vec![];
    for i in 0..4 {
        extracted_counters
            .push(RUNTIME.spawn_pinned_by_idx(|| async move { extract_data_from_thread() }, i));
    }
    for counter_future in extracted_counters {
        let extracted_counter = counter_future.await.unwrap_or_default();
        for (key, count) in extracted_counter {
            *complete_counter.entry(key).or_insert(0) += count;
        }
    }
    complete_counter
}

async fn something(number: u32) {
    tokio::time::sleep(std::time::Duration::from_secs(number as u64)).await;
    COUNTER.with(|counter| {
        let counter = unsafe { &mut *counter.get() };
        match counter.get_mut(&number) {
            Some(count) => {
                let placeholder = *count + 1;
                *count = placeholder;
            }
            None => {
                counter.insert(number, 1);
            }
        }
    });
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _handle = tokio::spawn(async {
        let sequence = [1, 2, 3, 4, 5];
        let repeated_sequence: Vec<_> = sequence.iter().cycle().take(500_000).cloned().collect();
        let mut futures = vec![];
        for number in repeated_sequence {
            futures.push(RUNTIME.spawn_pinned(move || async move {
                something(number).await;
                something(number).await;
            }))
        }
        for f in futures {
            f.await.unwrap();
        }
        println!("All futures complete");
    });

    let pid = std::process::id();
    println!("The PID of this process is: {pid}");
    let mut stream = signal(SignalKind::hangup()).unwrap();
    stream.recv().await;

    let complete_counter = get_complete_count().await;
    println!("Complete counter: {complete_counter:?}");
}
