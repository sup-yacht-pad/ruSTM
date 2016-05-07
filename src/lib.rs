mod transaction;
mod variable;
mod result;

#[cfg(test)]
mod test;

pub use variable::TVar;
pub use transaction::Transaction;
pub use result::*;
use std::sync::{Arc, Mutex};
use std::thread;

pub fn retry<T>() -> StmResult<T> {
    Err(StmError::Retry)
}

pub fn atomically<T, F>(f: F) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::run(f)
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
    let var = TVar::new("Hello World");
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
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct Node {
    val: i32,
    l: Option<Box<Node>>,
    r: Option<Box<Node>>,
}

impl Node {
    fn new(new_val: i32) -> Node {
        Node {
            val: new_val,
            l: None,
            r: None,
        }
    }

    fn insert(&mut self, new_val: i32) {
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

    fn size(&mut self) -> i32 {
        match (&mut self.l, &mut self.r) {
            (&mut None, &mut None) => 1,
            (&mut Some(ref mut subnodel), &mut Some(ref mut subnoder))=> subnodel.size() + subnoder.size() + 1,
            (&mut None, &mut Some(ref mut subnoder))=> subnoder.size() + 1,
            (&mut Some(ref mut subnodel), &mut None)=> subnodel.size() + 1,
        }
    }
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct Bst {
    root: Option<Box<Node>>,
}

impl Bst {
    pub fn new() -> Bst {
        Bst { root: None }
    }

    pub fn insert(&mut self, new_val: i32) {
        match self.root {
            None => self.root = Some(Box::new(Node::new(new_val))),
            Some(ref mut r) => r.insert(new_val),
        }
    }

    pub fn size(&mut self) -> i32 {
        match self.root {
            None => 0,
            Some(ref mut subnode) => subnode.size(),
        }
    }
}

#[test]
fn test_another_tree_insertion() {
    let mut children = vec![];
    let mut b = Bst::new();
    b.insert(5);
    let var = TVar::new(b);
    for x in 0..10 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             let y = if x == 5 {11} else {x};
             cur.insert(y);
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    assert_eq!(var.read_atomic().size(), 11);
    println!("This is the size of the tree after STM insertions: {}", var.read_atomic().size());
}

#[test]
fn test_another_tree_insertion_with_single_lock() {
    let b = Bst::new();
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    for x in 0..10 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        newb.insert(x);
        println!("Inserting this value for global lock : {}", x);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let mut newl = l.lock().unwrap();
    println!("This is the size of the tree after single lock insertions: {}", newl.size());
}