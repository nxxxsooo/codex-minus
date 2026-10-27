//! Desktop-independent runtime, also used by synchronous integration tests.
use std::future::Future;
use std::sync::OnceLock;

pub use tokio::task::{JoinHandle, spawn_blocking};

pub fn block_on<F: Future>(future: F) -> F::Output {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME
        .get_or_init(|| tokio::runtime::Runtime::new().expect("cannot initialize core runtime"))
        .block_on(future)
}
