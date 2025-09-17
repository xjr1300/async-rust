use tokio::sync::{
    mpsc::{Receiver, channel},
    oneshot,
};

struct RespMessage {
    value: usize,
    responder: oneshot::Sender<usize>,
}

async fn resp_actor(mut rx: Receiver<RespMessage>) {
    let mut state = 0;
    while let Some(msg) = rx.recv().await {
        state += msg.value;
        if msg.responder.send(state).is_err() {
            eprintln!("Failed o send response");
        }
    }
}

#[tokio::main]
async fn main() {
    // let max_count = 100_000_000;
    // let max_count = 50_000_000;
    let max_count = 10_000_000;
    // let max_count = 1_000_000;
    let (tx, rx) = channel::<RespMessage>(max_count);
    let _resp_actor_handle = tokio::spawn(async {
        resp_actor(rx).await;
    });

    let now = tokio::time::Instant::now();
    let mut handles = vec![];
    for i in 0..max_count {
        let cloned_tx = tx.clone();
        let future = async move {
            let (resp_tx, resp_rx) = oneshot::channel::<usize>();
            let msg = RespMessage {
                value: i,
                responder: resp_tx,
            };
            cloned_tx.send(msg).await.unwrap();
            let _ = resp_rx.await.unwrap();
        };
        handles.push(tokio::spawn(future));
    }

    for handle in handles {
        handle.await.unwrap();
    }
    println!("Elapsed: {:?}", now.elapsed());
}
