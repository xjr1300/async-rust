//! 永続的キーバリューストア
//!
//! `main`関数で`mpsc`チャネルを作成して、ルーターアクターにその受信側を渡して、ルーターアクターを起動(`spawn`）する。
//! このとき送信側は`ROUTER_SENDER`静的変数に束縛する。
//! キーバリューストアを操作するときは、`get`、`set`、`delete`関数を呼び出して`ROUTER_SENDER`を介して、
//! ルーターアクターに対してメッセージを送信する。
//!
//! ルーターアクター(`route`関数)は、最初に`mpsc`チャネルを作成して、キーバリューストアアクターにその受信側を渡して、
//! キーバリューストアアクターを起動する。
//! ルーターアクターがメッセージを受け取ったとき、上記チャネルの送信側からメッセージを送信して、メッセージをキーバリュー
//! ストアアクターに伝える。
//!
//! キーバリューストアアクター（`key_value_actor`関数）は、`mpsc`チャネルを作成して、永続用ファイルライタアクター
//! にその受信側を渡して、永続用ファイルライタアクターを起動する。
//!
//! また、永続用ファイルライタアクターと通信するワンショットチャネルを作成して、キーバリューストアが蓄積しているデータを
//! 返すようにメッセージを送信して、その応答を待機する。
//! 永続化ファイルライタアクターは、上記メッセージを受信したとき、ファイルからキーバリューストアが蓄積しているデータを
//! ファイルから読み込み、ワンショットチャネルを介してデータを返す。
//!
//! キーバリューストアアクターは、キーバリューストアのデータを受信した後、ルータアクターからのメッセージを受信し続ける
//! ループに入る。
//! このループ内で、ルーターアクターからメッセージを取得したとき、キーバリューストアが作成した`mpsc`チャネルを介して、
//! 永続用ファイルライタアクターに処理内容を送信する。
//! その後、キーバーリューストアデータを処理した後、ルーターアクターが作成した`mpsc`チャネルを介して結果を送信する。
//!
//! 永続用ファイルライタアクターは、キーバリューストアアクターからメッセージを受信した後、毎回ファイルをキーバリューストア
//! のデータで上書きして書き込む。
use std::{collections::HashMap, sync::OnceLock};

use tokio::{
    fs::File,
    io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{
        mpsc::{Receiver, Sender, channel},
        oneshot,
    },
};

static ROUTER_SENDER: OnceLock<Sender<RoutingMessage>> = OnceLock::new();

struct SetKeyValueMessage {
    key: String,
    value: Vec<u8>,
    response: oneshot::Sender<()>,
}

struct GetKeyValueMessage {
    key: String,
    response: oneshot::Sender<Option<Vec<u8>>>,
}

struct DeleteKeyValueMessage {
    key: String,
    response: oneshot::Sender<()>,
}

enum KeyValueMessage {
    Get(GetKeyValueMessage),
    Set(SetKeyValueMessage),
    #[allow(dead_code)]
    Delete(DeleteKeyValueMessage),
}

enum RoutingMessage {
    KeyValue(KeyValueMessage),
}

async fn key_value_actor(mut receiver: Receiver<KeyValueMessage>) {
    let (writer_key_value_sender, writer_key_value_receiver) = channel(32);
    tokio::spawn(writer_actor(writer_key_value_receiver));

    let (get_sender, get_receiver) = oneshot::channel();
    let _ = writer_key_value_sender
        .send(WriterLogMessage::Get(get_sender))
        .await;
    let mut map = get_receiver.await.unwrap();

    while let Some(message) = receiver.recv().await {
        if let Some(write_message) = WriterLogMessage::from_key_value_message(&message) {
            let _ = writer_key_value_sender.send(write_message).await;
        }
        match message {
            KeyValueMessage::Get(GetKeyValueMessage { key, response }) => {
                let _ = response.send(map.get(&key).cloned());
            }
            KeyValueMessage::Set(SetKeyValueMessage {
                key,
                value,
                response,
            }) => {
                map.insert(key, value);
                let _ = response.send(());
            }
            KeyValueMessage::Delete(DeleteKeyValueMessage { key, response }) => {
                map.remove(&key);
                let _ = response.send(());
            }
        }
    }
}

async fn router(mut receiver: Receiver<RoutingMessage>) {
    let (key_value_sender, key_value_receiver) = channel(32);
    tokio::spawn(key_value_actor(key_value_receiver));

    while let Some(message) = receiver.recv().await {
        match message {
            RoutingMessage::KeyValue(message) => {
                let _ = key_value_sender.send(message).await;
            }
        }
    }
}

async fn get(key: String) -> Result<Option<Vec<u8>>, std::io::Error> {
    let (tx, rx) = oneshot::channel();
    ROUTER_SENDER
        .get()
        .unwrap()
        .send(RoutingMessage::KeyValue(KeyValueMessage::Get(
            GetKeyValueMessage { key, response: tx },
        )))
        .await
        .unwrap();
    Ok(rx.await.unwrap())
}

async fn set(key: String, value: Vec<u8>) -> Result<(), std::io::Error> {
    // キーバリューストアにキーと値を保存したことを通知するワンショットチャネル
    let (tx, rx) = oneshot::channel();
    // キーバーリューストアに保存するキーと値とともに、上記のトランスミッターをMPSCチャネルを
    // 介して送信
    ROUTER_SENDER
        .get()
        .unwrap()
        .send(RoutingMessage::KeyValue(KeyValueMessage::Set(
            SetKeyValueMessage {
                key,
                value,
                response: tx,
            },
        )))
        .await
        .unwrap();
    // ワンショットチャネルからキーと値を保存したことを示す通知を受信
    rx.await.unwrap();
    Ok(())
}

#[allow(dead_code)]
async fn delete(key: String) -> Result<(), std::io::Error> {
    let (tx, rx) = oneshot::channel();
    ROUTER_SENDER
        .get()
        .unwrap()
        .send(RoutingMessage::KeyValue(KeyValueMessage::Delete(
            DeleteKeyValueMessage { key, response: tx },
        )))
        .await
        .unwrap();
    rx.await.unwrap();
    Ok(())
}

enum WriterLogMessage {
    Set(String, Vec<u8>),
    Delete(String),
    Get(oneshot::Sender<HashMap<String, Vec<u8>>>),
}

impl WriterLogMessage {
    fn from_key_value_message(message: &KeyValueMessage) -> Option<Self> {
        match message {
            KeyValueMessage::Get(_) => None,
            KeyValueMessage::Set(message) => Some(WriterLogMessage::Set(
                message.key.clone(),
                message.value.clone(),
            )),
            KeyValueMessage::Delete(message) => Some(WriterLogMessage::Delete(message.key.clone())),
        }
    }
}

impl From<&KeyValueMessage> for Option<WriterLogMessage> {
    fn from(message: &KeyValueMessage) -> Self {
        match message {
            KeyValueMessage::Get(_) => None,
            KeyValueMessage::Set(message) => Some(WriterLogMessage::Set(
                message.key.clone(),
                message.value.clone(),
            )),
            KeyValueMessage::Delete(message) => Some(WriterLogMessage::Delete(message.key.clone())),
        }
    }
}

async fn read_data_from_file(file_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut file = File::open(file_path).await?;
    println!("file was opened");
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    println!("file was read");
    let data: HashMap<String, Vec<u8>> = serde_json::from_str(&contents)?;
    println!("data was parsed");
    Ok(data)
}

async fn load_map(file_path: &str) -> HashMap<String, Vec<u8>> {
    match read_data_from_file(file_path).await {
        Ok(data) => {
            println!("Data loaded from file: {data:?}");
            data
        }
        Err(e) => {
            eprintln!("Failed to read from file: {e:?}");
            println!("Starting with an empty hashmap");
            HashMap::new()
        }
    }
}

async fn writer_actor(mut receiver: Receiver<WriterLogMessage>) -> io::Result<()> {
    let mut map = load_map("./data.json").await;
    let mut file = File::create("./data.json").await.unwrap();

    while let Some(message) = receiver.recv().await {
        match message {
            WriterLogMessage::Get(response) => {
                let _ = response.send(map.clone());
            }
            WriterLogMessage::Set(key, value) => {
                map.insert(key, value);
            }
            WriterLogMessage::Delete(key) => {
                map.remove(&key);
            }
        };
        let contents = serde_json::to_string(&map).unwrap();
        file.set_len(0).await?;
        file.seek(std::io::SeekFrom::Start(0)).await?;
        file.write_all(contents.as_bytes()).await?;
        file.flush().await?;
    }
    Ok(())
}

#[tokio::main]
#[allow(clippy::let_unit_value)]
async fn main() -> Result<(), std::io::Error> {
    // MPSCなルーティングチャネル
    let (sender, receiver) = channel(32);
    ROUTER_SENDER.set(sender).unwrap();
    tokio::spawn(router(receiver));

    let _ = set(String::from("hello"), b"world".to_vec()).await?;
    let value = get(String::from("hello")).await?;
    println!("{:?}", String::from_utf8(value.unwrap()));

    // let _ = delete(String::from("hello")).await?;
    // let value = get(String::from("hello")).await?;
    // println!("{value:?}");

    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}
