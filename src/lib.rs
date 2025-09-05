pub mod futures;
pub mod runtime;

/// spawn_taskを呼び出すマクロ
///
/// 優先度を指定しない場合は、FutureType::Lowに設定
#[macro_export]
macro_rules! spawn_task {
    ($future:expr) => {
        spawn_task!($future, $crate::runtime::FutureType::Low)
    };
    ($future:expr, $order:expr) => {
        $crate::runtime::spawn_task_function($future, $order)
    };
}

#[macro_export]
macro_rules! join {
    ($($future:expr),*) => {
        std::vec![$(futures_lite::future::block_on($future)),*]
    }
}

#[macro_export]
macro_rules! try_join {
    ($($future:expr),*) => {
        let mut results = std::vec![];
        $(
            let result = std::panic::catch_unwind(|| futures_lite::future::block_on($future););
            results.push(result);
        )*
        results
    }
}

#[macro_export]
macro_rules! try_join2 {
    ($t:ty; $($future:expr),* $(,)?) => {
        {
            let mut results: std::vec::Vec<std::thread::Result<$t>> = vec![];
            $(
                let result = std::panic::catch_unwind(|| futures_lite::future::block_on($future));
                results.push(result);
            )*
            results
        }
    }
}
