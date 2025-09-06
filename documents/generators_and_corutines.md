#  ジェネレーター（generator）とコルーチン（coroutine）

## ジェネレーター（genetator）

ジェネレーターは、反復処理（イテレーション）するために設計された特別な関数である。
ジェネレーター関数で`yield`キーワードを使用すると、その`yield`地点で関数は一時停止して、**呼び出し元の関数**に値を返し、呼び出し元の関数に制御を移動する。
そして、ジェネレーターは次のイテレーションまで待機する。
ジェネレーターは**外部からの呼び出しによってのみ再開**されるため、制御が他のジェネレーターやコルーチンに移動することはない。

例えば、次のようなジェネレーターを考える。

```python
def generator_func():
    print("Start generator")
    yield 1
    print("Yeilded 1")
    yield 2
    print("Yeilded 2")


gen = generator_func()
print(next(gen))
print(next(gen))
print(next(gen))
```

上記コードは、次を出力する。

```text
Start generator
1
Yielded 1
2
Yielded 2
Traceback (most recent call last):
  File "<stdin>", line 1, in <module>
StopIteration
```

`next(gen)`を呼び出すたびに、ジェネレーターは次の`yield`地点まで実行して値を返す。
そして、制御は呼び出し元に戻る。

## コルーチン（coroutine）

### 非同期処理

コルーチンは、ジェネレーターと同様に、反復処理のために設計された特別な関数であり、非同期処理を扱うために設計されている。

例えば、次のような非同期処理を考える。

```python
import asyncio

async def task_a():
    print("Task A starting...")
    await asyncio.sleep(1) # 1秒間待機
    print("Task A finished.")

async def task_b():
    print("Task B starting...")
    await asyncio.sleep(1) # 1秒間待機
    print("Task B finished.")

async def main():
    await asyncio.gather(task_a(), task_b())

asyncio.run(main())
```

上記コードを実行すると、`task_a`と`task_b`が同時並列に実行される。
`task_a`が`asyncio.sleep(1)`で待機状態になると、制御は`task_b`に移り`task_b`が実行される。
そして、`task_b`が`asyncio.sleep(1)`で待機状態になると、`task_a`に制御が移り、`print("Task A finished")`が実行される。
次は、`task_b`に処理が移り、残りのコードが実行される。

もし、`task_a`がジェネレーターであれば、`task_a`が`sleep`（`asyncio.sleep`でない）している間、プログラム全体が停止する。

コルーチンは、この実行制御の移動（コンテキストスイッチ）が自動的にされるため、効率よくタスクを実行できる。

### 呼び出し元の関数から値の受け取り

コルーチンは、ジェネレーターと同様に`yield`キーワードを使用して値を返すが、コルーチンは呼び出し元の関数に制御を移動するだけでなく、呼び出し元の関数から値を受け取ることもできる。

例えば、次のようなコルーチンを考える。

```python
def coroutine_func():
    print("Start coroutine")
    while True:
        x = yield
        print(f"Received: {x}")


coro = coroutine_func()
next(coro)
coro.send(1)
coro.send(2)
coro.send(3)
```

上記コードは、次を出力する。

```text
Start coroutine
Received: 1
Received: 2
Received: 3
```
