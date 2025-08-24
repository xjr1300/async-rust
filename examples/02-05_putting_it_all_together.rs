use std::{
    fs::{File, OpenOptions},
    future::Future,
    io::Write,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures_util::future::join_all;
use tokio::task::JoinHandle;

type AsyncFileHandle = Arc<Mutex<File>>;
type FileJoinHandle = JoinHandle<Result<bool, String>>;

fn get_handle(file_path: &dyn ToString) -> AsyncFileHandle {
    let file_path = file_path.to_string();
    match OpenOptions::new().append(true).open(&file_path) {
        Ok(opened_file) => Arc::new(Mutex::new(opened_file)),
        Err(_) => Arc::new(Mutex::new(
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&file_path)
                .unwrap(),
        )),
    }
}

struct AsyncWriteFuture {
    pub handle: AsyncFileHandle,
    pub entry: String,
}

impl Future for AsyncWriteFuture {
    type Output = Result<bool, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = match self.handle.try_lock() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("error for {} : {}", self.entry, e);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };
        let entry = format!("{}\n", self.entry);
        match guard.write_all(entry.as_bytes()) {
            Ok(_) => println!("written for: {}", self.entry),
            Err(e) => eprintln!("{e}"),
        };
        Poll::Ready(Ok(true))
    }
}

fn write_log(handle: AsyncFileHandle, entry: String) -> FileJoinHandle {
    let future = AsyncWriteFuture { handle, entry };
    tokio::task::spawn(future)
}

#[tokio::main]
async fn main() {
    let login_file_handle = get_handle(&"login.txt");
    let logout_file_handle = get_handle(&"logout.txt");

    let names = ["one", "two", "three", "four", "five", "six"];
    let mut handles = Vec::new();

    for name in names {
        let login_file_handle2 = Arc::clone(&login_file_handle);
        let logout_file_handle2 = Arc::clone(&logout_file_handle);
        let entry = name.to_string();
        let login_handle = write_log(login_file_handle2, entry.clone());
        let logout_handle = write_log(logout_file_handle2, entry);
        handles.push(login_handle);
        handles.push(logout_handle);
    }

    let _ = join_all(handles).await;
}
