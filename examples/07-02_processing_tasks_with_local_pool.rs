use std::cell::RefCell;
use tokio_util::task::LocalPoolHandle;

thread_local! {
    static COUNTER: RefCell<u32> = const {RefCell::new(1)};
}

async fn something(number: u32) -> u32 {
    // std::thread::sleep(std::time::Duration::from_secs(3));
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    COUNTER.with_borrow_mut(|counter| {
        *counter += 1;
        println!("Counter: {counter} for: {number}");
    });
    // COUNTER.with(|counter| {
    //     *counter.borrow_mut() += 1;
    //     println!("Counter: {} for: {number}", *counter.borrow());
    // });
    number
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pool = LocalPoolHandle::new(3);

    let one = pool.spawn_pinned_by_idx(
        || async {
            println!("one");
            something(1).await
        },
        0,
    );
    let two = pool.spawn_pinned_by_idx(
        || async {
            println!("two");
            something(2).await
        },
        0,
    );
    let three = pool.spawn_pinned_by_idx(
        || async {
            println!("three");
            something(3).await
        },
        0,
    );

    let result = async {
        let one = one.await.unwrap();
        let two = two.await.unwrap();
        let three = three.await.unwrap();
        one + two + three
    };
    println!("result: {}", result.await);
}
