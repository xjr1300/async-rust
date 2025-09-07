#![feature(coroutines, coroutine_trait)]
use std::{
    ops::{Coroutine, CoroutineState},
    pin::Pin,
    sync::{Arc, Mutex},
};

#[allow(dead_code)]
struct MutexCoroutine {
    handle: Arc<Mutex<u8>>,
    threshold: u8,
}

impl Coroutine<()> for MutexCoroutine {
    type Yield = ();
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, _arg: ()) -> CoroutineState<Self::Yield, Self::Return> {
        match self.handle.try_lock() {
            Ok(mut handle) => {
                *handle += 1;
            }
            Err(_) => {
                return CoroutineState::Yielded(());
            }
        }
        self.threshold -= 1;
        if self.threshold == 0 {
            CoroutineState::Complete(())
        } else {
            CoroutineState::Yielded(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        task::{Context, Poll},
        time::Duration,
    };

    // 同期テストのインターフェイス
    fn check_yield(coroutine: &mut MutexCoroutine) -> bool {
        match Pin::new(coroutine).resume(()) {
            CoroutineState::Yielded(_) => true,
            CoroutineState::Complete(_) => false,
        }
    }

    // 非同期ランタイムのインターフェイス
    impl Future for MutexCoroutine {
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

    #[test]
    fn basic_test() {
        let handle = Arc::new(Mutex::new(0));
        let mut first_coroutine = MutexCoroutine {
            handle: handle.clone(),
            threshold: 2,
        };
        let mut second_coroutine = MutexCoroutine {
            handle: handle.clone(),
            threshold: 2,
        };

        let lock = handle.lock().unwrap();
        for _ in 0..3 {
            assert_eq!(check_yield(&mut first_coroutine), true);
            assert_eq!(check_yield(&mut second_coroutine), true);
        }
        assert_eq!(*lock, 0);
        std::mem::drop(lock);

        assert_eq!(check_yield(&mut first_coroutine), true);
        assert_eq!(*handle.lock().unwrap(), 1);
        assert_eq!(check_yield(&mut second_coroutine), true);
        assert_eq!(*handle.lock().unwrap(), 2);
        assert_eq!(check_yield(&mut first_coroutine), false);
        assert_eq!(*handle.lock().unwrap(), 3);
        assert_eq!(check_yield(&mut second_coroutine), false);
        assert_eq!(*handle.lock().unwrap(), 4);
    }

    #[tokio::test]
    async fn async_test() {
        let handle = Arc::new(Mutex::new(0));
        let first_coroutine = MutexCoroutine {
            handle: handle.clone(),
            threshold: 2,
        };
        let second_coroutine = MutexCoroutine {
            handle: handle.clone(),
            threshold: 2,
        };

        let handle_one = tokio::spawn(async move {
            first_coroutine.await;
        });
        let handle_two = tokio::spawn(async move {
            second_coroutine.await;
        });
        handle_one.await.unwrap();
        handle_two.await.unwrap();
        assert_eq!(*handle.lock().unwrap(), 4);
    }
}

fn main() {}
