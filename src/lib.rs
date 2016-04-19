mod transaction;
mod variable;
mod res;

pub use transaction::Transaction;
pub use res::{StmError, StmResult};

use std::sync::atomic::AtomicUsize;

pub fn atomically<T, F>(f: F, lock: AtomicUsize) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::run(f, lock)
}

