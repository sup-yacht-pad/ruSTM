mod transaction;
mod variable;
mod result;

#[cfg(test)]
mod test;

pub use variable::TVar;
pub use transaction::Transaction;
pub use result::*;
use std::sync::Arc;
use std::thread;

/// call `retry`, to abort an operation. It takes another path of an
/// `Transaction::or` or blocks until any variable changes.
///
/// # Examples
///
/// ```no_run
/// use stm::*;
/// let infinite_retry: i32 = atomically(|_| retry());
/// ```
pub fn retry<T>() -> StmResult<T> {
    Err(StmError::Retry)
}

/// Run a function atomically by using Software Transactional Memory.
/// It calls to `Transaction::with` internally, but is more explicit.
pub fn atomically<T, F>(f: F) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::with(f)
}

#[test]
fn test_infinite_retry() {
    let terminated = test::terminates(300, || { 
        let _infinite_retry: i32 = atomically(|_| retry());
    });
    assert!(!terminated);
}

#[test]
fn test_stm_nested() {
    let var = TVar::new(0);

    let x = atomically(|trans| {
        try!(var.write(trans, 42));
        var.read(trans)
    });

    assert_eq!(42, x);
}

/// Run multiple threads.
///
/// Thread 1: Read a var, block until it is not 0 and then
/// return that value.
///
/// Thread 2: Wait a bit. Then write a value.
///
/// Check if Thread 1 is woken up correctly and then check for 
/// correctness.
#[test]
fn test_threaded() {
    use std::thread;
    use std::time::Duration;

    let var = TVar::new(0);
    let var_ref = var.clone();

    let x = test::async(800,
        move || {
            atomically(|trans| {
                let x = try!(var_ref.read(trans));
                if x == 0 {
                    retry()
                } else {
                    Ok(x)
                }
            })
        },
        move || {
            thread::sleep(Duration::from_millis(100));

            atomically(|trans| var.write(trans, 42));
        }
    ).unwrap();

    assert_eq!(42, x);
}

/// test if a STM calculation is rerun when a Var changes while executing
#[test]
fn test_read_write_interfere() {
    use std::thread;
    use std::time::Duration;

    // create var
    let var = TVar::new(0);
    let var_ref = var.clone();

    // spawn a thread
    let t = thread::spawn(move || {
        atomically(|log| {
            // read the var
            let x = try!(var_ref.read(log));
            // ensure that x var_ref changes in between
            thread::sleep(Duration::from_millis(500));

            // write back modified data this should only
            // happen when the value has not changed
            var_ref.write(log, x + 10)
        });
    });

    // ensure that the thread has started and already read the var
    thread::sleep(Duration::from_millis(100));

    // now change it
    atomically(|trans| var.write(trans, 32));

    // finish and compare
    let _ = t.join();
    assert_eq!(42, var.read_atomic());
}

#[test]
fn test_simple() {
    let var = Arc::new(TVar::new("Hello World"));
    for _ in 0..10 {
        let newvar = var.clone();
        thread::spawn(move || {
            let x = atomically(|trans| {
             try!(newvar.write(trans, "Oh no"));
             try!(newvar.write(trans, "Help me"));
             try!(newvar.write(trans, "Vincent sucks"));
             newvar.read(trans) // return the value saved in var
            });
        });
    }
    // let x = atomically(|trans| {
    //  try!(var.write(trans, "Oh no"));
    //  try!(var.write(trans, "Help me"));
    //  try!(var.write(trans, "Vincent sucks"));
    //  var.read(trans) // return the value saved in var
    // });
    //assert_eq!(x, "Vincent sucks");
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct Node<'a> {
    val: &'a str,
    l: Option<Box<Node<'a>>>,
    r: Option<Box<Node<'a>>>,
}
impl<'a> Node<'a> {
    pub fn insert(&mut self, new_val: &'a str) {
        if self.val == new_val {
            return
        }
        let target_node = if new_val < self.val { &mut self.l } else { &mut self.r };
        match target_node {
            &mut Some(ref mut subnode) => subnode.insert(new_val),
            &mut None => {
                let new_node = Node { val: new_val, l: None, r: None };
                let boxed_node = Some(Box::new(new_node));
                *target_node = boxed_node;
            }
        }
    }
}

#[test]
fn test_another_simple() {
    let var = TVar::new(Node { val: "m", l: None, r: None });
    let x = atomically(|trans| {
     let mut y = try!(var.read(trans));
     y.insert("z");
     y.insert("b");
     y.insert("c");
     try!(var.write(trans, y));
     var.read(trans) // return the value saved in var
    });
    assert_eq!(x, Node {
        val: "m",
        l: Some(Box::new(Node {
            val: "b",
            l: None,
            r: Some(Box::new(Node { val: "c", l: None, r: None })),
        })),
        r: Some(Box::new(Node { val: "z", l: None, r: None })),
    });
}