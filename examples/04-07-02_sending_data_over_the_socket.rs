use std::{
    error::Error,
    future::Future,
    io::{self, Read, Write},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_lite::future;
use mio::{
    Events, Interest, Poll as MioPoll, Token,
    net::{TcpListener, TcpStream},
};

use async_rust::{runtime::Runtime, spawn_task};

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

struct ServerFuture {
    server: TcpListener,
    poll: MioPoll,
}

impl Future for ServerFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // ソケットからイベントを取り出し
        let mut events = Events::with_capacity(1);
        self.poll
            .poll(&mut events, Some(Duration::from_millis(200)))
            .unwrap();
        // 取り出したイベントを処理
        for event in events.iter() {
            if event.token() == SERVER && event.is_readable() {
                // ソケットリスナーがデータを受け付け
                let (mut stream, _) = self.server.accept().unwrap();
                // ソケットリスナーからデータを取得
                // ソケットリスナーからは、1024バイトずつデータを取得するが、ソケットリスナーが受け付けた
                // データが1024バイトよりも大きいこともあるため、received_dataの容量を拡張しながら
                // ループですべてのデータを取得する。
                let mut buffer = [0u8; 1024];
                let mut received_data = vec![];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(n) if n > 0 => {
                            received_data.extend_from_slice(&buffer[..n]);
                        }
                        Ok(_) => {
                            break;
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                            // WouldBlockは、「現在読み取れるデータがない」を意味するが、それまでにデータを
                            // 読み取っている場合がある。
                            // したがって、WouldBlockが発生した場合は、単純にbreakして、データ読み取りループを
                            // 脱出する。
                            // もし、次のようにした場合、OK(_)のアームに到達しないため、Poll::Ready(T)が返されない。
                            // この場合、エグゼキューターによるpollメソッドの呼び出しが無限に繰り返される。
                            // ```rust
                            // cx.waker().wake_by_ref();
                            // return Poll::Pending;
                            // ```
                        }
                        Err(e) => {
                            eprintln!("Error reading from stream: {e}");
                            break;
                        }
                    }
                }
                if !received_data.is_empty() {
                    // データを受け取ったらソケットリスナーを終了
                    let received_str = String::from_utf8_lossy(&received_data);
                    return Poll::Ready(received_str.to_string());
                    /*
                    // ソケットのリスナーを修了せず、無限にイベントを処理する場合
                    async_rust::spawn_task!(some_async_handle_function(&received_data)).detach();
                    return Poll::Pending;
                     */
                } else {
                    // ソケットリスナーが受け付けたデータがからの場合は、Wakerを登録して、再度ポーリングしてもらう
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    Runtime::new().with_low_num(2).with_high_num(4).run();

    // サーバー
    let addr = "127.0.0.1:13265".parse()?;
    let mut server = TcpListener::bind(addr)?;
    let mut stream = TcpStream::connect(server.local_addr()?)?;
    let poll: MioPoll = MioPoll::new()?;
    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;
    let server_worker = ServerFuture { server, poll };
    let test = spawn_task!(server_worker);

    // クライアント
    let mut client_poll: MioPoll = MioPoll::new()?;
    client_poll
        .registry()
        .register(&mut stream, CLIENT, Interest::WRITABLE)?;
    let mut events = Events::with_capacity(128);
    client_poll.poll(&mut events, None).unwrap();
    for event in events.iter() {
        if event.token() == CLIENT && event.is_writable() {
            let message = "that's so dingo\n";
            let _ = stream.write_all(message.as_bytes());
        }
    }

    let outcome = future::block_on(test);
    println!("outcome: {outcome}");

    Ok(())
}
