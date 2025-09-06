#![feature(coroutines, coroutine_trait)]

use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader},
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

struct ReadCoroutine {
    lines: io::Lines<BufReader<File>>,
}

impl ReadCoroutine {
    fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(file);
        let lines = reader.lines();
        Ok(Self { lines })
    }
}

/// ```
/// pub trait Coroutine<R = ()> {
///     type Yield;
///     type Return;
///
///     // Required method
///     fn resume(
///         self: Pin<&mut Self>,
///         arg: R,
///     ) -> CoroutineState<Self::Yield, Self::Return>;
/// }
/// ```
///
/// ジェネリックパラメーター`R`は、`resume`メソッドを呼び出すときの引数の型を指定する。
impl Coroutine<()> for ReadCoroutine {
    type Yield = i32;
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, _arg: ()) -> CoroutineState<Self::Yield, Self::Return> {
        match self.lines.next() {
            Some(Ok(line)) => {
                if let Ok(number) = line.parse::<i32>() {
                    CoroutineState::Yielded(number)
                } else {
                    CoroutineState::Complete(())
                }
            }
            Some(Err(_)) | None => CoroutineState::Complete(()),
        }
    }
}

fn main() -> io::Result<()> {
    let mut coroutine = ReadCoroutine::new("./numbers.txt")?;
    while let CoroutineState::Yielded(number) = Pin::new(&mut coroutine).resume(()) {
        println!("{number}");
    }
    Ok(())
}
