#![feature(coroutines, coroutine_trait)]

use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Write},
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

struct WriteCoroutine {
    writer: BufWriter<File>,
}

impl WriteCoroutine {
    fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }
}

impl Coroutine<i32> for WriteCoroutine {
    type Yield = ();
    type Return = ();

    fn resume(mut self: Pin<&mut Self>, arg: i32) -> CoroutineState<Self::Yield, Self::Return> {
        if let Err(e) = writeln!(self.as_mut().writer, "{arg}") {
            eprintln!("Failed to write a number: {e}");
        }
        CoroutineState::Yielded(())
    }
}

struct CoroutineManager {
    reader: ReadCoroutine,
    writer: WriteCoroutine,
}

impl CoroutineManager {
    fn new(read_path: &str, write_path: &str) -> io::Result<Self> {
        Ok(Self {
            reader: ReadCoroutine::new(read_path)?,
            writer: WriteCoroutine::new(write_path)?,
        })
    }

    fn run(&mut self) {
        let mut read_pin = Pin::new(&mut self.reader);
        let mut write_pin = Pin::new(&mut self.writer);
        while let CoroutineState::Yielded(number) = read_pin.as_mut().resume(()) {
            write_pin.as_mut().resume(number);
        }
    }
}

fn main() -> io::Result<()> {
    let mut manager = CoroutineManager::new("numbers.txt", "output.txt")?;
    manager.run();
    Ok(())
}
