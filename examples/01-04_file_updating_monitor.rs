use std::path::PathBuf;

use tokio::{
    fs::File as AsyncFile,
    io::AsyncReadExt,
    sync::watch,
    time::{Duration, sleep},
};

async fn read_file(filename: &str) -> Result<String, std::io::Error> {
    let mut file = AsyncFile::open(filename).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    Ok(contents)
}

async fn watch_file_changes(tx: watch::Sender<bool>) {
    let path = PathBuf::from("data.txt");
    let mut last_modified = None;
    loop {
        if let Ok(metadata) = path.metadata() {
            let modified = metadata.modified().unwrap();
            if last_modified != Some(modified) {
                last_modified = Some(modified);
                let _ = tx.send(true);
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    let (tx, mut rx) = watch::channel(false);

    tokio::spawn(watch_file_changes(tx));

    loop {
        // ファイルが変更されるまで待機
        let _ = rx.changed().await;

        // ファイルを読み込み、コンソールにその内容を出力

        if let Ok(contents) = read_file("data.txt").await {
            println!("{contents}")
        }
    }
}
