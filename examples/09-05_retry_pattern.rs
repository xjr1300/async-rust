async fn get_data() -> Result<String, Box<dyn std::error::Error>> {
    Err("Error".into())
}

async fn do_something() -> Result<(), Box<dyn std::error::Error>> {
    let mut milliseconds = 1000;
    let total_count = 5;
    let mut count = 0;
    let _result: String;
    loop {
        match get_data().await {
            Ok(data) => {
                _result = data;
                break;
            }
            Err(err) => {
                eprintln!("Error: {err}");
                count += 1;
                if count == total_count {
                    return Err(err);
                }
            }
        }
        println!("waiting for {milliseconds} milliseconds");
        tokio::time::sleep(tokio::time::Duration::from_millis(milliseconds)).await;
        milliseconds *= 2;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let outcome = do_something().await;
    println!("Outcome: {outcome:?}");
}
