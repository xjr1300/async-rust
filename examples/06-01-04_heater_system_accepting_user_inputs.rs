use std::{
    future::Future,
    io::{self, Write},
    pin::Pin,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicI16, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

use device_query::{DeviceEvents, DeviceState};

// デバイスへの入力
static INPUT: LazyLock<Arc<Mutex<String>>> = LazyLock::new(|| Arc::new(Mutex::new(String::new())));
// デバイスの状態
static DEVICE_STATE: LazyLock<Arc<DeviceState>> = LazyLock::new(|| Arc::new(DeviceState::new()));

fn render(temp: i16, desired_temp: i16, heat_on: bool, input: String) {
    clearscreen::clear().unwrap();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    println!(
        "Temperature: {}\nDesired Temp: {}\nHeater On: {heat_on}",
        temp as f32 / 100.0,
        desired_temp as f32 / 100.0
    );
    print!("Input: {input}");
    handle.flush().unwrap();
}

// 現在の温度
static TEMP: LazyLock<Arc<AtomicI16>> = LazyLock::new(|| Arc::new(AtomicI16::new(2090)));
// 設定温度
static DESIRED_TEMP: LazyLock<Arc<AtomicI16>> = LazyLock::new(|| Arc::new(AtomicI16::new(2100)));
// ヒーターをONにするかを示すフラグ
static HEAT_ON: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));

struct DisplayFuture {
    temp_snapshot: i16,
}

impl DisplayFuture {
    fn new() -> Self {
        DisplayFuture {
            // TEMPへの書き込み（store）操作があった場合、書き込み操作の後で次を実行して、
            // 書き込み操作前のすべてのメモリ操作が実行された後で、TEMPの値を読み取り（load）
            temp_snapshot: TEMP.load(Ordering::SeqCst),
        }
    }
}

impl Future for DisplayFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current_snapshot = TEMP.load(Ordering::SeqCst);
        let desired_temp = DESIRED_TEMP.load(Ordering::SeqCst);
        let heat_on = HEAT_ON.load(Ordering::SeqCst);

        // 現在の温度と同じ場合は何もするこないため、何もせずに再度ポーリングされるようにウェイカーを登録
        // ※設定温度が変更されない限り
        if current_snapshot == self.temp_snapshot {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // ヒーターをONするかOFFするか決定
        if current_snapshot < desired_temp && !heat_on {
            // 現在の温度が設定温度よりも低い場合は、ヒーターをON
            HEAT_ON.store(true, Ordering::SeqCst);
        } else if current_snapshot > desired_temp && heat_on {
            // 現在の温度が設定温度よりも高い場合は、ヒーターをOFF
            HEAT_ON.store(false, Ordering::SeqCst);
        }

        clearscreen::clear().unwrap();
        println!(
            "Temperature: {}\nDesired Temp: {}\nHeater On: {}",
            current_snapshot as f32 / 100.0,
            desired_temp as f32 / 100.0,
            heat_on
        );

        // 最新の温度を保存して、再度ポーリングされるようにウェイカーを登録
        self.temp_snapshot = current_snapshot;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

struct HeaterFuture {
    time_snapshot: Instant,
}

impl HeaterFuture {
    fn new() -> Self {
        Self {
            time_snapshot: Instant::now(),
        }
    }
}

impl Future for HeaterFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // ヒーターがONでない場合は、温度が上がらないため終了
        if !HEAT_ON.load(Ordering::SeqCst) {
            self.time_snapshot = Instant::now();
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // 前回pollが呼び出されたときから、3秒未満しか経過していない場合は、単位時間あたりにヒーターを操作
        // する頻度が多いため終了（ヒーターをONにした効果が現れていない）
        let current_snapshot = Instant::now();
        if current_snapshot.duration_since(self.time_snapshot) < Duration::from_secs(3) {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // 温度を上げる
        TEMP.fetch_add(3, Ordering::SeqCst);
        self.time_snapshot = Instant::now();
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

struct HeatLossFuture {
    time_snapshot: Instant,
}

impl HeatLossFuture {
    fn new() -> Self {
        Self {
            time_snapshot: Instant::now(),
        }
    }
}

impl Future for HeatLossFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current_snapshot = Instant::now();
        if current_snapshot.duration_since(self.time_snapshot) >= Duration::from_secs(3) {
            TEMP.fetch_sub(1, Ordering::SeqCst);
            self.time_snapshot = Instant::now();
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[tokio::main]
async fn main() {
    let _guard = DEVICE_STATE.on_key_down(|key| {
        let mut input = INPUT.lock().unwrap(); // INPUTをロック
        input.clear();
        // input.push_str(&key.to_string());
        *input = key.to_string();
        std::mem::drop(input); // INPUTのロックを解放
        render(
            TEMP.load(Ordering::SeqCst),
            DESIRED_TEMP.load(Ordering::SeqCst),
            HEAT_ON.load(Ordering::SeqCst),
            INPUT.lock().unwrap().clone(),
        );
    });
    let display = tokio::spawn(async {
        DisplayFuture::new().await;
    });
    let heat_loss = tokio::spawn(async {
        HeatLossFuture::new().await;
    });
    let heater = tokio::spawn(async {
        HeaterFuture::new().await;
    });
    display.await.unwrap();
    heat_loss.await.unwrap();
    heater.await.unwrap();
}
