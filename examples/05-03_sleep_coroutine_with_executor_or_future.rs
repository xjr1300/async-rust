#![feature(coroutines, coroutine_trait)]
use std::{
    collections::VecDeque,
    future::Future,
    ops::{Coroutine, CoroutineState},
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::executor::block_on;

struct SleepCoroutine {
    pub start: Instant,
    pub duration: Duration,
}

impl SleepCoroutine {
    fn new(duration: Duration) -> Self {
        Self {
            start: Instant::now(),
            duration,
        }
    }
}

impl Coroutine<()> for SleepCoroutine {
    type Yield = ();
    type Return = ();

    fn resume(self: Pin<&mut Self>, _arg: ()) -> CoroutineState<Self::Yield, Self::Return> {
        if self.start.elapsed() >= self.duration {
            CoroutineState::Complete(())
        } else {
            CoroutineState::Yielded(())
        }
    }
}

impl Future for SleepCoroutine {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self).resume(()) {
            CoroutineState::Yielded(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            CoroutineState::Complete(_) => Poll::Ready(()),
        }
    }
}

type PinnedCoroutine = Pin<Box<dyn Coroutine<(), Yield = (), Return = ()>>>;
type CoroutineVecDeque = VecDeque<PinnedCoroutine>;

#[derive(Default)]
struct Executor {
    coroutines: CoroutineVecDeque,
}

impl Executor {
    fn add(&mut self, coroutine: PinnedCoroutine) {
        self.coroutines.push_back(coroutine);
    }

    fn poll(&mut self) {
        println!("Polling {} coroutines", self.coroutines.len());
        let mut coroutine = self.coroutines.pop_front().unwrap();
        match coroutine.as_mut().resume(()) {
            CoroutineState::Yielded(_) => {
                self.coroutines.push_back(coroutine);
            }
            CoroutineState::Complete(_) => {}
        }
    }
}

fn main() {
    let mut executor = Executor::default();
    for _ in 0..3 {
        let coroutine = SleepCoroutine::new(Duration::from_secs(1));
        executor.add(Box::pin(coroutine));
    }

    let start = Instant::now();
    while !executor.coroutines.is_empty() {
        executor.poll();
    }
    println!("Took {:?}", start.elapsed());

    println!("----------");

    println!("Call SleepCoroutine future");
    let start = Instant::now();
    let coroutine = SleepCoroutine::new(Duration::from_secs(1));
    block_on(coroutine);
    println!("Took {:?}", start.elapsed());
}
