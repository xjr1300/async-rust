use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Write},
    pin::Pin,
};

trait SymmetricCoroutine {
    type Input;
    type Output;

    fn resume_with_input(self: Pin<&mut Self>, input: Self::Input) -> Self::Output;
}

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

impl SymmetricCoroutine for ReadCoroutine {
    type Input = ();
    type Output = Option<i32>;

    fn resume_with_input(mut self: Pin<&mut Self>, _input: Self::Input) -> Self::Output {
        match self.as_mut().lines.next() {
            Some(Ok(line)) => match line.parse::<i32>() {
                Ok(number) => Some(number),
                Err(e) => {
                    eprintln!("Failed to parse a line: {e}");
                    None
                }
            },
            Some(Err(e)) => {
                eprintln!("Failed to read a line: {e}");
                None
            }
            None => None,
        }
    }
}

struct WriteCoroutine {
    writer: BufWriter<File>,
}

impl WriteCoroutine {
    fn new(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }
}

impl SymmetricCoroutine for WriteCoroutine {
    type Input = i32;
    type Output = ();

    fn resume_with_input(mut self: Pin<&mut Self>, input: Self::Input) -> Self::Output {
        if let Err(e) = writeln!(self.as_mut().writer, "{input}") {
            eprintln!("Failed to write a number: {e}");
        }
    }
}

fn main() -> io::Result<()> {
    let mut reader = ReadCoroutine::new("numbers.txt")?;
    let mut writer = WriteCoroutine::new("output.txt")?;
    while let Some(number) = Pin::new(&mut reader).resume_with_input(()) {
        Pin::new(&mut writer).resume_with_input(number);
    }
    Ok(())
}
