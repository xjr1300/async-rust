#![feature(coroutines, coroutine_trait)]
use std::{
    ops::{Coroutine, CoroutineState},
    pin::Pin,
    time::Duration,
};

use rand::Rng;

struct RandCoroutine {
    pub value: u8,
    pub live: bool,
}

impl RandCoroutine {
    fn new() -> Self {
        let mut coroutine = Self {
            value: 0,
            live: true,
        };
        coroutine.generate();
        coroutine
    }

    fn generate(&mut self) {
        let mut rng = rand::rng();
        self.value = rng.random_range(0..=127);
    }
}

impl Coroutine<()> for RandCoroutine {
    type Yield = u8;
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, _arg: ()) -> CoroutineState<Self::Yield, Self::Return> {
        self.generate();
        CoroutineState::Yielded(self.value)
    }
}

fn main() {
    let mut coroutines = vec![];
    for _ in 0..10 {
        coroutines.push(RandCoroutine::new());
    }

    let mut total: u32 = 0;

    loop {
        let mut all_dead = true;
        for mut coroutine in coroutines.iter_mut() {
            if coroutine.live {
                all_dead = false;
                match Pin::new(&mut coroutine).resume(()) {
                    CoroutineState::Yielded(value) => {
                        total += value as u32;
                    }
                    CoroutineState::Complete(()) => {
                        panic!("Coroutine should not complete");
                    }
                }
                if coroutine.value < 64 {
                    coroutine.live = false;
                }
            }
        }
        if all_dead {
            break;
        }
    }
    println!("Total: {total}");

    let (transmitter, receiver) = std::sync::mpsc::channel::<RandCoroutine>();
    let _thread = std::thread::spawn(move || {
        while let Ok(coroutine) = receiver.recv() {
            let mut coroutine = coroutine;
            match Pin::new(&mut coroutine).resume(()) {
                CoroutineState::Yielded(number) => {
                    println!("Coroutine yielded: {number}");
                }
                CoroutineState::Complete(()) => {
                    panic!("Coroutine should not complete");
                }
            }
        }
    });

    // 上記スレッドが起動することを待つ
    std::thread::sleep(Duration::from_secs(1));
    transmitter.send(RandCoroutine::new()).unwrap();
    transmitter.send(RandCoroutine::new()).unwrap();
    // コルーチンがいくつか値を生成することを待つ
    std::thread::sleep(Duration::from_secs(1));
}
