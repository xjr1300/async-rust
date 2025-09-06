#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

use std::{
    fs::OpenOptions,
    io::{self, BufRead, BufReader},
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

fn main() -> io::Result<()> {
    // `||`内には、`resume`メソッドの引数を指定する。
    // 今回の場合は、何も受け取らないため引数を指定しておらず、`resume`メソッドを呼び出すときの引数に`()`を指定している。
    let mut coroutine = #[coroutine]
    || {
        let file = OpenOptions::new().read(true).open("numbers.txt")?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        while let Some(Ok(line)) = lines.next() {
            if let Ok(number) = line.parse::<i32>() {
                yield number;
            } else {
                break;
            }
        }
        Ok::<(), io::Error>(())
    };
    while let CoroutineState::Yielded(number) = Pin::new(&mut coroutine).resume(()) {
        println!("{number}");
    }
    Ok(())
}
