pub mod runtime;

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
        vec![$(futures_lite::future::block_on($future)),*]
    }
}

#[macro_export]
macro_rules! try_join {
    ($($future:expr),*) => {
        let mut results = vec![];
        $(
            let result = catch_unwind(|| future::block_on($future););
            results.push(result);
        )*
        results
    }
}
