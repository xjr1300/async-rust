use std::time::{Duration, Instant};

use serde::Deserialize;
use tokio::time::sleep;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Response {
    url: String,
    args: serde_json::Value,
}

async fn fetch_data(seconds: u64) -> Result<Response, reqwest::Error> {
    let request_url = format!("https://httpbin.org/delay/{seconds}");
    let response = reqwest::get(&request_url).await?;
    let delayed_response = response.json::<Response>().await?;
    Ok(delayed_response)
}

async fn calculate_last_login() {
    sleep(Duration::from_secs(1)).await;
    println!("Logged in 2 days ago");
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let start_time = Instant::now();
    let data = fetch_data(5);
    let time_since = calculate_last_login();
    let (posts, _) = tokio::join!(data, time_since);
    let duration = start_time.elapsed();
    println!("Fetched {posts:?}");
    println!("Time taken: {duration:?}");
    Ok(())
}
