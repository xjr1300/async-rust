type WaterfallResult = Result<String, Box<dyn std::error::Error>>;

async fn task1() -> WaterfallResult {
    Ok("Task 1 completed".to_string())
}

async fn task2(input: String) -> WaterfallResult {
    Ok(format!("{input} then Task 2 completed"))
}

async fn task3(input: String) -> WaterfallResult {
    Ok(format!("{input} and finally Task 3 completed"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output1 = task1().await?;
    let output2 = task2(output1).await?;
    let output3 = task3(output2).await?;
    println!("{output3}");
    Ok(())
}
