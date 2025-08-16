use std::ops::Deref;
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/*
std::sync::Condvar

Condition variables represent the ability to block a thread such that it consumes no CPU time while waiting for an event to occur.
条件変数は、イベントの発生を待機している間に、CPU時間を消費しないようにスレッドをブロックする機能を表現する。

Condition variables are typically associated with a boolean predicate (a condition) and a mutex.
通常、条件変数は、ブーリアン述語（条件）とミューテックスと関連づけられる。

The predicate is always verified inside of the mutex before determining that a thread must block.
スレッドをブロックしなければならないかを決定する前に、常に述語はミューテックスの内部で検証される。

Functions in this module will block the current thread of execution.
このモジュール内の関数は、現在実行しているカレントスレッドをブロックする。

Note that any attempt to use multiple mutexes on the same condition variable may result in a runtime panic.
同じ条件変数を複数のミューテックスで使用することを試みは、ランタイムでパニックする可能性がある。
 */
fn main() {
    let shared_data = Arc::new((Mutex::new(false), Condvar::new()));
    let cloned_shared_data = Arc::clone(&shared_data);
    let stop = Arc::new(AtomicBool::new(false));
    let cloned_stop = Arc::clone(&stop);

    let _background_thread = thread::spawn(move || {
        let (mutex, condvar) = cloned_shared_data.deref();
        let mut received_value = mutex.lock().unwrap();
        // stopの初期値はfalseであるため、whileループに侵入
        while !cloned_stop.load(Ordering::Relaxed) {
            // Condvar::waitメソッドは、ロック済みのMutexGuardを受け取る。
            // まず、受け取ったMutexGuardの所有権を奪い、内部でドロップすることでMutexをアンロックする。
            // これにより他のスレッドがMutexをロックできるようになる。
            //     updater_threadの`*mutex.lock().unwrap() = update_value;`を実行できるようになる。
            // その後、現在のスレッド（background_thread）は、条件変数の待機状態に入る。
            //     updater_threadが`condvar.notify_one();`を実行するまで（イベントが発生するまで）待機状態が続く。
            //     このときスレッドはOSによってブロックされ、CPU資源を消費しない。
            // イベントが発生した時、待機していたスレッドが起床して、再び同じMutexをロックする。
            // ロックに成功すると、新しいMutexGuardが得られ、呼び出し元に戻り値として返す。
            // この新しいMutexGuardは、waitメソッドに渡したMutexGuardと同じMutexをロックしている。
            // したがって、このスレッドで共有データを安全にアクセスできるMutexGuardを継続的に利用できる。
            received_value = condvar.wait(received_value).unwrap();
            println!("Received value: {}", *received_value);
        }
    });

    let updater_thread = thread::spawn(move || {
        let (mutex, condvar) = shared_data.deref();
        let values = [false, true, false, true];

        for update_value in values {
            println!("Updating value to {update_value} ...");
            *mutex.lock().unwrap() = update_value;
            condvar.notify_one();
            thread::sleep(Duration::from_secs(4));
        }
        // バックグラウンドスレッドのwhileループを終了するためにstopフラグを成立させる
        stop.store(true, Ordering::Relaxed);
        println!("The stop has been updated");
        condvar.notify_one();
    });
    updater_thread.join().unwrap();
}
