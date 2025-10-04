# 10.2.1 Wakerの自作

- `Waker`は非同期タスクを「起こす（wake）」ための仕組み
- 内部的には`RawWaker`を持っており、その操作方法を`RawWakerVTable`という「関数ポインタの表」で定義
- ランタイム側は、どのようにタスクを再度スケジューリングするか」を`wake`関数や`wake_by_ref`関数などに実装して、`Future`を駆動させる

## Wakerの作成

```rust
pub fn create_raw_waker() -> RawWaker {
    let data = Box::into_raw(Box::new(42u32));
    RawWaker::new(data as *const (), &VTABLE)
}
```

- `RawWaker`は、任意のデータを保持でき、ここでは`u32`型のデータを保持
- `42u32`という値を`Box`でラップして、ヒープに確保
  - `Box::new(42u32)`
  - `RawWaker`が生きている間は有効なポインタでなくてはならないため、データをヒープに確保する必要があるため`Box`でラップ
  - ヒープのデータは明示的に`drop`されるまで生き続ける
- `Box::into_raw`によって、所有権をBoxからムーブして生ポインタ（`*mut u32`）を取得した後、`*const ()`にキャストして、`RawWaker`を作成
  - `RawWaker::new(ptr: *const (), vtable: &RawWakerVTable)`
  - `*const ()`は任意の型の値のポインタを保持する型（`void`ポインタ）

## VTableの内容

```rust
static VTABLE: RawWakerVTable = RawWakerVTable::new(my_clone, my_wake, my_wake_by_ref, my_drop);
```

- `RawWakerVTable::new`は、その`Waker`をどのように操作するかを定義

### my_clone

```rust
unsafe fn my_clone(raw_waker: *const ()) -> RawWaker {
    RawWaker::new(raw_waker, &VTABLE)
}
```

- クローンするときに、同じデータを指す`RawWaker`を返す
- 本来は、`Arc`などで参照カウンタを増やすことが普通であるが、ここでは同じデータを指し示すポインタをコピーして`RawWaker`を複製

### my_wake

```rust
unsafe fn my_wake(raw_waker: *const ()) {
    drop(unsafe { Box::from_raw(raw_waker as *mut u32) });
}
```

- `wake`が呼ばれたとき、`Box::from_raw`でポインタを`Box<u32>`に戻す
- その後、`drop`してメモリを解放
- つまり起こされると、すぐにタスクが完了するために、ヒープに確保した領域を解放

### my_wake_by_ref

```rust
unsafe fn my_wake_by_ref(_raw_waker: *const ()) {}
```

- `wake_by_ref`は「所有権を消費せずに起こす」バージョン
- ここでは何もしない

### my_drop

```rust
unsafe fn my_drop(raw_waker: *const ()) {
    drop(unsafe { Box::from_raw(raw_waker as *mut u32) });
}
```

- `RawWaker`が`drop`されたときに呼び出される処理
- `wake`と同様にメモリを解放

## Wakerのライフサイクル

- `Waker`は内部に`RawWaker`を保持
- `wake`が呼ばれると、その`Waker`は消費される（`self`をムーブ）ので、その時点で`drop`は呼ばれない
  - `wake`の中で`Waker`を解放
  - つまり、`drop`は呼ばれない
- `drop`が呼ばれるのは`wake`されなかった`Waker`のみであるため、`wake`と`drop`による二重解放は発生しない
- また、`wake_by_ref`では`Waker`は借用されているため解放されないため、後で`drop`される

## Arcを使用した実際のランタイムの実装

```rust
use std::{
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

struct Task {
    // 実際には Future やスケジューラの情報を持つ
}

fn clone_arc(ptr: *const ()) -> RawWaker {
    let arc = unsafe { Arc::<Task>::from_raw(ptr as *const Task) };
    let arc2 = arc.clone();
    std::mem::forget(arc); // 参照カウントを減らさない
    RawWaker::new(Arc::into_raw(arc2) as *const (), &VTABLE)
}

fn wake_arc(ptr: *const ()) {
    let arc = unsafe { Arc::<Task>::from_raw(ptr as *const Task) };
    // スケジューラに登録する処理など…
    drop(arc); // wake は所有権を奪うので drop で参照カウントを減らす
}

fn wake_by_ref_arc(ptr: *const ()) {
    let arc = unsafe { Arc::<Task>::from_raw(ptr as *const Task) };
    // スケジューラに登録する処理など…
    std::mem::forget(arc); // 参照カウントは減らさない
}

fn drop_arc(ptr: *const ()) {
    let arc = unsafe { Arc::<Task>::from_raw(ptr as *const Task) };
    drop(arc); // 最後の参照なら解放される
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_arc, wake_arc, wake_by_ref_arc, drop_arc);

fn arc_waker(task: Arc<Task>) -> Waker {
    let raw = RawWaker::new(Arc::into_raw(task) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw) }
}
```
