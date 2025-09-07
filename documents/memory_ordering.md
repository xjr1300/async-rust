# メモリ順序（Memory Ordering）

メモリ順序とは、**マルチスレッド環境下でアトミック操作（原子性を持つ操作）が、他のメモリアクセスとどのように順序付けられるかを指定する仕組み**である。
現在のCPUは性能向上のためにメモリアクセスの順序を最適化して並び替えることがあり、これがマルチスレッドプログラムで予期しない動作を引き起こす可能性がある。

## Ordering::Relaxed（最も弱い順序保証）

- 操作対象の変数に対しては原子性を保証する
- 他のメモリアクセスとの順序関係は保証しない
- 処理性能は高いが、同期保証は最小限

```rust
static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);

// スレッド1
std::thread::spawn(|| {
    X.store(1, Ordering::Relaxed);  // ①
    Y.store(1, Ordering::Relaxed);  // ②
});

// スレッド2
std::thread::spawn(|| {
    let y = Y.load(Ordering::Relaxed);
    let x = X.load(Ordering::Relaxed);
    println!("x = {x}, y = {y}");
})
```

`Ordering::Relaxed`は、**原子性**は保証するが、**順序性**は保証しない。
したがって、上記コードでは①と②の実行順序がスレッド2から見て逆転して見える可能性がある。

- 得られる可能性のある結果は、(x = 0, y = 0), (x = 1, y = 0), (x = 0, y = 1), (x = 1, y = 1)の4種類
- (x = 0, y = 0)
  - スレッド2がスレッド1よりも先に実行された場合
- (x = 1, y = 0)
  - スレッド1の`X.store(1)`は完了したが、`Y.store(1)`が完了していない状態でスレッド2が完了した場合
- (x = 0, y = 1)
  - `Ordering::Relaxed`の特徴的なケース
  - スレッド1の`Y.store(1)`が先に実行され完了したが、`X.store(1)`が完了していない状態でスレッド2が完了した場合
  - または、メモリキャッシュの影響で、スレッド2から`Y`の変更が先に見えた場合
- (x = 1, y = 1)
  - スレッド1の両方の書き込みが完了した後で、スレッド2が値を読み取った場合

## Ordering::SeqCst（最も強い順序保証）

- すべてのスレッド見て、すべての`Ordering::SecCst`操作が1つの順序で並ぶ
- `store-load`ペアを**同期点**として扱い、他のメモリ操作の順序も保証する
- プログラム全体で、直感的に**逐次実行されたかのように**見える

```rust
static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);

// スレッド1
std::thread::spawn(|| {
    X.store(1, Ordering::SeqCst);
    Y.store(1, Ordering::SeqCst);
});

// スレッド2
std::thread::spawn(|| {
    let y = Y.load(Ordering::SeqCst);
    let x = X.load(Ordering::SeqCst);
    println!("x = {x}, y = {y}");
})
```

- 得られうる結果は、(x = 0, y = 0), (x = 1, y = 0), (x = 1, y = 1)の3種類
- (x = 0, y = 1)という結果は絶対に起こらない
- 上記コードにおいて、`y`が`1`であれば、`x`も`1`であることが保証される
  - スレッド2で`y`を読み取った値が`1`であれば、スレッド1の処理順番が保証されているため、`x`も`1`である

## store-loadペアとメモリ同期

`store-load`ペアとは、あるスレッドにおける`store`（書き込み）操作と、別のスレッドにおける`load`（読み込み）操作のペアである。
これらのペアは、**同期点**として機能して、`store`前のすべての操作は、対応する`load`後のすべての操作より前に観測される。

### Ordering::Relaxedの場合

```rust
use std::sync::atomic::{AtomicI32, Ordering};

static FLAG: AtomicI32 = AtomicI32::new(0);
static DATA: AtomicI32 = AtomicI32::new(0);

// スレッド1
thread::spawn(|| {
    DATA.store(42, Ordering::Relaxed);    // ① データ書き込み
    FLAG.store(1, Ordering::Relaxed);     // ② フラグ設定
});

// スレッド2
thread::spawn(|| {
    if FLAG.load(Ordering::Relaxed) == 1 { // ③ フラグ確認
        let data = DATA.load(Ordering::Relaxed); // ④ データ読み取り
        println!("data = {}", data); // data = 0の可能性がある
    }
});
```

- ③で`FLAG == 1`を観測しても、④で`DATA == 42`が観測される保証はない
- ②と③の`store-load`ペアは`DATA`の順序を保証しない

`Ordering::Relaxed`の場合、次の問題がある。

### Ordering::Relaxedの場合

```rust
// スレッド1
thread::spawn(|| {
    DATA.store(42, Ordering::SeqCst);    // ① データ書き込み
    FLAG.store(1, Ordering::SeqCst);     // ② フラグ設定
});

// スレッド2
thread::spawn(|| {
    if FLAG.load(Ordering::SeqCst) == 1 { // ③ フラグ確認
        let data = DATA.load(Ordering::SeqCst); // ④ データ読み取り
        println!("data = {}", data); // 必ず data = 42
    }
});
```

- ②と③の`store-load`ペアがスレッド間の同期点として機能
- ①を含む②以前の操作が、③以降のすべての操作より前に観測される
- したがって、①→②→③→④の順序が保証される

```text
スレッド1: [① DATA=42] [② FLAG=1]
           ↓synchronized↓
スレッド2:              [③ FLAG読み取り] [④ DATA読み取り]
                       ↑
                  ②③ペアが同期点となり
                  ①→④の順序も保証される
```

`Ordering::SeqCst`の場合、複数のスレッドからなる命令があり、その中で同じ変数に対する保存と読み取りがあった場合、もしその保存が実行された場合、その保存の前のすべての操作が実行されており、その保存の後でその変数に対する読み取り以降の操作が実行される。

```rust
static FLAG: AtomicI32 = AtomicI32::new(0);
static DATA1: AtomicI32 = AtomicI32::new(0);
static DATA2: AtomicI32 = AtomicI32::new(0);

// スレッド1
DATA1.store(10, Ordering::SeqCst);  // A
DATA2.store(20, Ordering::SeqCst);  // B
FLAG.store(1, Ordering::SeqCst);    // C (store)

// スレッド2
if FLAG.load(Ordering::SeqCst) == 1 { // D (load)
    let d1 = DATA1.load(Ordering::SeqCst); // E
    let d2 = DATA2.load(Ordering::SeqCst); // F
}
```

- 上記で`C`が実行された場合、必ず`D`以後の処理は`C`の後で実行される
- したがって、`C`が実行された場合、必ず`d1 = 10`、`d2 = 20`になる
- 上記で`C`が実行されていない場合、必ず`d1 = 0`、`d2 = 0`になる

`C(store)`と`D(load)`がペアになった瞬間、次の順番が保証される。

```text
[C以前の全操作] → [D以降の全操作]
     ↓               ↓
   A, B           →  E, F
```
