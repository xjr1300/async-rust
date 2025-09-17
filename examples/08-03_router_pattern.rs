use std::sync::OnceLock;

use tokio::sync::{
    mpsc::{Receiver, Sender, channel},
    oneshot,
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
}

async fn key_value_actor(mut receiver: Receiver<KeyValueMessage>) {
    let mut map = std::collections::HashMap::new();
    while let Some(message) = receiver.recv().await {
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
    let _ = delete(String::from("hello")).await?;
    let value = get(String::from("hello")).await?;
    println!("{value:?}");

    Ok(())
}
