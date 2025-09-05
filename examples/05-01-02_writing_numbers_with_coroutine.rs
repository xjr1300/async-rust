#![feature(coroutines, coroutine_trait)]

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    ops::{Coroutine, CoroutineState},
    pin::Pin,
    time::Instant,
};

use rand::Rng;

struct WriteCoroutine {
    pub file_handle: File,
}

impl WriteCoroutine {
    fn new(path: &str) -> io::Result<Self> {
        let file_handle = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file_handle })
    }
}

impl Coroutine<i32> for WriteCoroutine {
    type Yield = ();
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, arg: i32) -> CoroutineState<Self::Yield, Self::Return> {
        writeln!(self.file_handle, "{arg}").unwrap();
        CoroutineState::Yielded(())
    }
}

fn main() -> io::Result<()> {
    let mut rng = rand::rng();
    let numbers: Vec<_> = (0..200_000).map(|_| rng.random::<i32>()).collect();
    let mut coroutine = WriteCoroutine::new("numbers.txt")?;

    let start = Instant::now();
    for number in numbers {
        Pin::new(&mut coroutine).resume(number);
    }
    let duration = start.elapsed();

    println!("Time elapsed in file operations is: {duration:?}");
    Ok(())
}
