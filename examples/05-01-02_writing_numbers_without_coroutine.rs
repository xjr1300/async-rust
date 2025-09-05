#![feature(coroutines, coroutine_trait)]

use std::{
    fs::OpenOptions,
    io::{self, Write},
    time::Instant,
};

use rand::Rng;

fn append_number_to_file(n: i32) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("numbers.txt")?;
    writeln!(file, "{n}")
}

fn main() -> io::Result<()> {
    let mut rng = rand::rng();
    let numbers: Vec<_> = (0..200_000).map(|_| rng.random::<i32>()).collect();

    let start = Instant::now();
    for number in numbers {
        if let Err(e) = append_number_to_file(number) {
            eprintln!("Failed to write to file: {e}");
        }
    }
    let duration = start.elapsed();

    println!("Time elapsed in file operations is: {duration:?}");
    Ok(())
}
