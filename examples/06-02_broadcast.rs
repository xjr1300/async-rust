use std::{
    collections::{HashMap, VecDeque},
    marker::Send,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use tokio::sync::Mutex as AsyncMutex;

/// `T`はイベントを表現する型であるため、複数のサブスクライバーにイベントを届けるためには、
/// `Clone`を実装していなければならない。また、イベントはスレッドをまたぐ可能性があるため
/// `Send`を実装していなくてはならない。
struct EventBus<T: Clone + Send> {
    /// u32型の値で識別されるサブスクライバに対するイベントキュー
    /// 非同期タスク間でデータを共有するため、tokio版のMutexで保護する。
    /// tokio版のMutexは、ロックを取得するとき非同期で待機するため、ロックの取得を待っている
    /// 間、他のタスクを処理できる。
    /// std版のMutexは、ロックを取得するときスレッドをブロックするため、ロックの取得を待っている
    /// タスクが停止して、他のタスクを処理できない。
    chamber: AsyncMutex<HashMap<u32, VecDeque<T>>>,
    /// サブスクライバーに採番する番号
    count: AtomicU32,
    /// 購読を解除したサブスクライバー
    /// サブスクライバーが購読を解除したときにアクセスされるため、std版のMutexで保護する。
    /// また、イベントキューからサブスクライバーを削除するときは、イベントキューから他のタスクからの
    /// アクセスをブロックできるため、同期がｋ簡単になる。
    dead_ids: Mutex<Vec<u32>>,
}

impl<T: Clone + Send> EventBus<T> {
    fn new() -> Self {
        Self {
            chamber: AsyncMutex::new(HashMap::new()),
            count: AtomicU32::new(0),
            dead_ids: Mutex::new(Vec::new()),
        }
    }

    async fn subscribe(&self) -> EventHandle<'_, T> {
        let mut chamber = self.chamber.lock().await;
        let id = self.count.fetch_add(1, Ordering::SeqCst);
        chamber.insert(id, VecDeque::new());
        EventHandle {
            id,
            // EventBusの参照をEventHandleに渡す
            // event_bus: Arc::new(self),
            event_bus: self,
        }
    }

    fn unsubscribe(&self, id: u32) {
        self.dead_ids.lock().unwrap().push(id);
    }

    async fn poll(&self, id: u32) -> Option<T> {
        let mut chamber = self.chamber.lock().await;
        match chamber.get_mut(&id) {
            Some(queue) => queue.pop_front(),
            None => None,
        }
    }

    async fn send(&self, event: T) {
        let mut chamber = self.chamber.lock().await;
        for (_, queue) in chamber.iter_mut() {
            queue.push_back(event.clone());
        }
    }
}

/// EventHandleのライフタイムは、EventBusのライフタイムよりも短い
struct EventHandle<'a, T: Clone + Send> {
    id: u32,
    // event_bus: Arc<&'a EventBus<T>>,
    event_bus: &'a EventBus<T>,
}

impl<'a, T: Clone + Send> EventHandle<'a, T> {
    async fn poll(&self) -> Option<T> {
        self.event_bus.poll(self.id).await
    }
}

impl<'a, T: Clone + Send> Drop for EventHandle<'a, T> {
    fn drop(&mut self) {
        self.event_bus.unsubscribe(self.id);
    }
}

async fn consume_event_bus(event_bus: Arc<EventBus<f32>>) {
    let handle = event_bus.subscribe().await;
    loop {
        let event = handle.poll().await;
        match event {
            Some(event) => {
                println!("id: {}, value: {event}", handle.id);
                if event == 3.0 {
                    break;
                }
            }
            None => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

async fn garbage_collector(event_bus: Arc<EventBus<f32>>) {
    loop {
        let mut chamber = event_bus.chamber.lock().await;
        let dead_ids = event_bus.dead_ids.lock().unwrap().clone();
        event_bus.dead_ids.lock().unwrap().clear();
        for id in dead_ids.iter() {
            chamber.remove(id);
        }
        std::mem::drop(chamber);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[tokio::main]
async fn main() {
    // イベントバスを作成
    let event_bus = Arc::new(EventBus::<f32>::new());
    // コンシューマーを作成
    let bus_one = Arc::clone(&event_bus);
    let bus_two = Arc::clone(&event_bus);
    let gb_bus_ref = Arc::clone(&event_bus);

    let _gb = tokio::task::spawn(async { garbage_collector(gb_bus_ref).await });
    let one = tokio::task::spawn(async { consume_event_bus(bus_one).await });
    let two = tokio::task::spawn(async { consume_event_bus(bus_two).await });

    std::thread::sleep(Duration::from_secs(1));

    event_bus.send(1.0).await;
    event_bus.send(2.0).await;
    event_bus.send(3.0).await;

    let _ = one.await;
    let _ = two.await;
    println!("{:?}", event_bus.chamber.lock().await);
    std::thread::sleep(Duration::from_secs(3));
    println!("{:?}", event_bus.chamber.lock().await);
}
