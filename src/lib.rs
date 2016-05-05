mod transaction;
mod variable;
mod res;

pub use variable::TVari32;
pub use transaction::Transaction;
pub use res::{StmError, StmResult};
use std::sync::Arc;
use std::thread;

pub fn atomically<T, F>(f: F) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::run(f)
}

#[test]
fn test_another_simple() {
    let var = TVari32::new(52);
    println!("this is the original address: {0:x}", var.get_addr());
    let x = atomically(|trans| {
     try!(var.write(trans, 5));
     var.read(trans) // return the value saved in var
    });
    assert_eq!(x, 5);
    assert_eq!(var.read_atomic(), 52);
}

// fn test_transaction_copy() {
//     let read = TVari32::new(42);
//     let write = TVari32::new(0);

//     Transaction::run(|trans| {
//         let r = try!(read.read(trans));
//         let l = try!(write.read(trans));
//         write.write(trans, r);
//         read.write(trans, l)
//     });
//     assert_eq!(write.read_atomic(), 0); //this is clearly wrong :(
//     assert_eq!(read.read_atomic(), 42);
// }

// fn test_simple() {
//     let var = Arc::new(TVari32::new(5));
//     for _ in 0..100 {
//         let newvar = var.clone();
//         thread::spawn(move || {
//             let x = atomically(|trans| {
//              try!(newvar.write(trans, 2));
//              try!(newvar.write(trans, 1));
//              newvar.read(trans) // return the value saved in var
//             });
//         });
//     }
//     assert_eq!(var.read_atomic(), 5);
// }