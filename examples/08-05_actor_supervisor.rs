use std::{collections::HashMap, sync::OnceLock};

use tokio::{
    fs::File,
    io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{
        mpsc::{Receiver, Sender, channel},
        oneshot,
    },
    time::{self, Duration, Instant},
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
    Delete(DeleteKeyValueMessage),
}

enum RoutingMessage {
    KeyValue(KeyValueMessage),
    Heartbeat(ActorType),
    Reset(ActorType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ActorType {
    KeyValue,
    Writer,
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
            KeyValueMessage::Set(message) => {
                Some(Self::Set(message.key.clone(), message.value.clone()))
            }
            KeyValueMessage::Delete(message) => Some(Self::Delete(message.key.clone())),
        }
    }
}

async fn read_data_from_file(file_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut file = File::open(file_path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    let data: HashMap<String, Vec<u8>> = serde_json::from_str(&contents)?;
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

async fn key_value_actor(mut receiver: Receiver<KeyValueMessage>) {
    println!("Starting key_value_actor");
    let (writer_key_value_sender, writer_key_value_receiver) = channel(32);
    tokio::spawn(writer_actor(writer_key_value_receiver));

    let (get_sender, get_receiver) = oneshot::channel();
    let _ = writer_key_value_sender
        .send(WriterLogMessage::Get(get_sender))
        .await;
    let mut map = get_receiver.await.unwrap();

    let timeout_duration = Duration::from_millis(200);
    let router_sender = ROUTER_SENDER.get().unwrap().clone();

    loop {
        match time::timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(message)) => {
                if let Some(write_message) = WriterLogMessage::from_key_value_message(&message) {
                    let _ = writer_key_value_sender.send(write_message).await;
                }
                match message {
                    KeyValueMessage::Get(GetKeyValueMessage { key, response }) => {
                        let _ = response.send(map.get(&key).cloned());
                    }
                    KeyValueMessage::Delete(DeleteKeyValueMessage { key, response }) => {
                        map.remove(&key);
                        let _ = response.send(());
                    }
                    KeyValueMessage::Set(SetKeyValueMessage {
                        key,
                        value,
                        response,
                    }) => {
                        map.insert(key, value);
                        let _ = response.send(());
                    }
                }
            }
            Ok(None) => break,
            Err(_) => {
                router_sender
                    .send(RoutingMessage::Heartbeat(ActorType::KeyValue))
                    .await
                    .unwrap();
            }
        }
    }
}

async fn writer_actor(mut receiver: Receiver<WriterLogMessage>) -> io::Result<()> {
    time::sleep(Duration::from_millis(100)).await;
    println!("Starting writer_actor");

    let mut map = load_map("./data.json").await;
    let mut file = File::create("./data.json").await.unwrap();

    let timeout_duration = Duration::from_millis(200);
    let router_sender = ROUTER_SENDER.get().unwrap().clone();
    let now = Instant::now();

    loop {
        println!("writer instance: {now:?}");
        match time::timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(message)) => {
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
            Ok(None) => break,
            Err(_) => {
                router_sender
                    .send(RoutingMessage::Heartbeat(ActorType::Writer))
                    .await
                    .unwrap();
            }
        };
    }
    Ok(())
}

async fn heartbeat_actor(mut receiver: Receiver<ActorType>) {
    println!("Starting heartbeat_actor");
    let mut map = HashMap::new();
    let timeout_duration = Duration::from_millis(200);
    let now = Instant::now();

    loop {
        match time::timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(actor_name)) => map.insert(actor_name, Instant::now()),
            Ok(None) => break,
            Err(_) => continue,
        };

        let little_second_ago = Instant::now() - Duration::from_millis(700);
        let mut key_reset = false;
        for (key, &value) in map.iter() {
            if value < little_second_ago {
                match key {
                    ActorType::KeyValue | ActorType::Writer => {
                        println!("sending reset message form {now:?} {key:?}");
                        ROUTER_SENDER
                            .get()
                            .unwrap()
                            .send(RoutingMessage::Reset(ActorType::KeyValue))
                            .await
                            .unwrap();
                        key_reset = true;
                    }
                }
            }
            if key_reset {
                break;
            }
        }
        if key_reset {
            map.remove(&ActorType::KeyValue);
            map.remove(&ActorType::Writer);
        }
    }
}

async fn router_actor(mut receiver: Receiver<RoutingMessage>) {
    println!("Starting router_actor");

    let (mut key_value_sender, mut key_value_receiver) = channel(32);
    let mut key_value_handle = tokio::spawn(key_value_actor(key_value_receiver));

    let (heartbeat_sender, heartbeat_receiver) = channel(32);
    tokio::spawn(heartbeat_actor(heartbeat_receiver));

    while let Some(message) = receiver.recv().await {
        match message {
            RoutingMessage::KeyValue(message) => {
                let _ = key_value_sender.send(message).await;
            }
            RoutingMessage::Heartbeat(message) => {
                let _ = heartbeat_sender.send(message).await;
            }
            RoutingMessage::Reset(message) => match message {
                ActorType::KeyValue | ActorType::Writer => {
                    let (new_key_value_sender, new_key_value_receiver) = channel(32);
                    key_value_handle.abort();
                    key_value_sender = new_key_value_sender;
                    key_value_receiver = new_key_value_receiver;
                    key_value_handle = tokio::spawn(key_value_actor(key_value_receiver));
                    time::sleep(Duration::from_millis(100)).await;
                }
            },
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

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let (sender, receiver) = channel(32);
    ROUTER_SENDER.set(sender).unwrap();
    tokio::spawn(router_actor(receiver));

    set("hello".to_string(), b"world".to_vec()).await?;

    let value = get("hello".to_string()).await;
    println!("value: {value:?}");

    ROUTER_SENDER
        .get()
        .unwrap()
        .send(RoutingMessage::Reset(ActorType::KeyValue))
        .await
        .unwrap();

    let value = get("hello".to_string()).await?;
    println!("value: {value:?}");

    set("test".to_string(), b"world".to_vec()).await?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    Ok(())
}
