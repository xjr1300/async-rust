//! コルーチンリテラルについては、次を参照すること。
//! <https://doc.rust-lang.org/beta/unstable-book/language-features/coroutines.html>
#![feature(coroutines, coroutine_trait, stmt_expr_attributes)]

use std::{
    fs::OpenOptions,
    io::{self, Write},
    ops::Coroutine as _,
    pin::Pin,
    time::Instant,
};

use rand::Rng;

fn main() -> io::Result<()> {
    let mut rng = rand::rng();
    let numbers: Vec<_> = (0..200_000).map(|_| rng.random::<i32>()).collect();

    let mut write_coroutine = #[coroutine]
    |_v| {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("numbers.txt")?;
        loop {
            // コルーチンはループごとにここで停止する。
            // resumeメソッドを呼び出したとき、呼び出したときの引数がyieldに束縛される。
            let tmp = yield;
            writeln!(file, "{tmp}")?;
        }
        #[allow(unreachable_code)]
        Ok::<(), io::Error>(())
    };

    let start = Instant::now();
    for number in numbers {
        // コルーチンは他のメモリ位置に移動させてはならないためピン留め
        Pin::new(&mut write_coroutine).resume(number);
    }
    let duration = start.elapsed();

    println!("Time elapsed in file operations is: {duration:?}");
    Ok(())
}
