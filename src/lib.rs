mod transaction;
mod variable;
mod res;

pub use variable::TVari32;
pub use transaction::Transaction;
pub use res::{StmError, StmResult};

pub fn atomically<T, F>(f: F) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::run(f)
}

#[test]
fn test_another_simple() {
    let var = TVari32::new(52);
    let x = atomically(|trans| {
     try!(var.write(trans, 5));
     var.read(trans) // return the value saved in var
    });
    assert_eq!(x, 5);
}