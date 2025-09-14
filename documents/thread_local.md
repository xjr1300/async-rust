# thread_local!マクロ

`thread_Local!`マクロは、スレッドごとに独立したグローバルなデータを持ちたいときに使用する。
この独立したデータは、スレッドごとのスレッドローカルストレージ（Thread Local Storage、TLS）に格納される。

通常の`static`変数は、プロセス全体で1つか存在しない。
しかし、マルチスレッドプログラムにおいて、スレッドそれぞれで別々な静的な変数を持ちたいが場合がある。

- それぞれのスレッドが専用のキャッシュを持つ
- ロギングでスレッドごとにIDを割り当てて、ログを出力する
- データベースコネクションをそれぞれのスレッドごとに持つ

```rust
use std::cell::RefCell;

thread_local! {
    static COUNTER: RefCell<u32> = RefCell::new(0);
}
```

上記`COUNTER`はスレッドごとに独立した`RefCell<u32>`になる。
なお、`RefCell<T>`を使用している理由は、内部可変性を使用して可変にするためである。
単純に`static mut COUNTER: u32 = 0;`のようにグローバルな可変な静的変数にすることもできるが、そのアクセスには`unsafe`ブロックが必要である。

スレッドローカルストレージの変数にアクセスするためには`with`メソッドを使用する。

```rust
COUNTER.with(|c| {
    *c.borrow_mut() += 1;
    println!("COUNTER = {}", *c.borrow());
});
```

スレッドローカル変数にアクセスするキーは、`std::thread::LocalKey<T>`で、`thread_local!`マクロは、スレッドローカルストレージに値を登録するとともに、その値にアクセスするキーを作成する。
次のコードで`COUNTER`の型は`std::thread::LocalKey<RefCell<u32>>`である。

```rust
thread_local! {
    const COUNTER: std::thread::LocalKey<RefCell<u32>> = RefCell::new(0);
}
```

`LocalKey<T>`は、`with`メソッドでクロージャーを取る理由は、安全性のためである。

- スレッド終了時に、スレッドローカルストレージは破棄されるため、`&'static T`を返すことはできない
- `with`メソッドに渡したクロージャーのスコープで`&T`を有効にすることで、アクセスを安全にする
