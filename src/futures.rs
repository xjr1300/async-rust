use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

/// カウンターを持つFuture
///
/// pollが呼ばれるたびにカウンターをインクリメントし、カウンターが3になったら完了する。
#[derive(Debug, Clone, Copy)]
pub struct CounterFuture {
    count: u32,
    max: u32,
}

impl CounterFuture {
    pub fn new(count: u32, max: u32) -> Self {
        Self { count, max }
    }
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.count += 1;
        println!("polling with result: {}", self.count);

        // 本来、poll内でブロッキング処理をするのは非推奨
        std::thread::sleep(Duration::from_secs(1));

        if self.count < self.max {
            // wakerを呼び出して、再度pollされるように登録
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

pub async fn async_fn() {
    std::thread::sleep(Duration::from_secs(1));
    println!("async_fn");
}
