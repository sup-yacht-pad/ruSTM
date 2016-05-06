use std::collections::BTreeMap;
use std::collections::btree_map::Entry::*;
use std::mem;
use std::sync::{Arc};
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use super::variable::{TVar, VarControlBlock};
use super::result::*;
use super::result::StmError::*;

type ArcAny = Arc<Any + Send + Sync>;

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;
pub struct Transaction {
    snapshot: usize,
    writevars: BTreeMap<Arc<VarControlBlock>, ArcAny>,
    readvars: BTreeMap<Arc<VarControlBlock>, ArcAny>,
}

impl Transaction {
    fn new(ss: usize) -> Transaction {
        Transaction { 
            snapshot: ss,
            writevars: BTreeMap::new(),
            readvars: BTreeMap::new()
        }
    }

    pub fn with<T, F>(f: F) -> T 
    where F: Fn(&mut Transaction) -> StmResult<T>,
    {
        let mut ss = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
        while (ss & 1) != 0 {
            ss = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
        }
        let mut transaction = Transaction::new(ss);

        loop {
            match f(&mut transaction) {
                Ok(t) => {
                    if transaction.commit() {
                        return t;
                    }
                }
                Err(_) => { }
            }
            transaction.clear();
        }
    }

    fn downcast<T: Any + Clone>(var: Arc<Any>) -> T {
        var.downcast_ref::<T>()
           .expect("Vars with different types and same address")
           .clone()
    }

    pub fn read<T: Send + Sync + Any + Clone>(&mut self, var: &TVar<T>) -> StmResult<T> {
        let ctrl = var.control_block().clone();
        match self.writevars.get(&ctrl) {

            Some(mut entry) => {let value = entry.clone();
                                    return Ok(Transaction::downcast(value));}
            None => { }
        }
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            match self.validate() {
                None => { return Err(Retry); }
                Some(ss) => {
                    self.snapshot = ss;
                }
            }
        }
        let value = var.read_ref_atomic();
        let ctrl = var.control_block().clone();
        self.readvars.insert(ctrl, value.clone());
        //self.readvars.insert(ctrl, LogVar::Read(value.clone()));
        Ok(Transaction::downcast(value))
    }

    pub fn write<T: Any + Send + Sync + Clone>(&mut self, var: &TVar<T>, value: T) -> StmResult<()> {
        let boxed = Arc::new(value);
        let ctrl = var.control_block().clone();
        self.writevars.insert(ctrl, boxed);
        Ok(())
    }

    fn clear(&mut self) {
        self.writevars.clear();
        self.readvars.clear();
        let mut ss = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
        while (ss & 1) != 0 {
            ss = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
        }
        self.snapshot = ss;
    }

    fn validate(&mut self) -> Option<usize> {
        loop {
            let time = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
            if time & 1 != 0 {
                continue;
            }
            let vars = mem::replace(&mut self.readvars, BTreeMap::new());
            let mut read_vec = Vec::new();
            for (var, value) in &vars {
                match value {
                    // &Write(ref w) | &ReadObsoleteWrite(_,ref w) => { }
                    // &ReadWrite(ref original,ref w) => { }
                    // &ReadObsolete(_) => { }
                    //&Read(ref original) 
                        ref original => {
                        let lock = var.value.read().unwrap();
                        if !same_address(&lock, original) {
                            mem::drop(read_vec);
                            return None;
                        }
                        read_vec.push(lock);
                    }
                }
            }
            mem::drop(read_vec);
            if time == GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
                return Some(time);
            }
        }
    }


    fn commit(&mut self) -> bool {
        if self.writevars.is_empty() {
            return true;
        }
        while GLOBAL_SEQ_LOCK.compare_and_swap(self.snapshot, self.snapshot+1, Ordering::SeqCst) != self.snapshot {
            match self.validate() {
                None => { return false; }
                Some(ss) => { self.snapshot = ss; }
            }
        }
        let vars = mem::replace(&mut self.writevars, BTreeMap::new());
        //let mut read_vec = Vec::new();
        let mut write_vec = Vec::new();

        for (var, value) in &vars {
            match value {
                ref val => {
                    let lock = var.value.write().unwrap();
                    write_vec.push((var, val.clone(), lock));
                }
                // // We need to take a write lock.
                // &Write(ref w) | &ReadObsoleteWrite(_,ref w)=> {
                //     // take write lock
                //     let lock = var.value.write().unwrap();
                //     // add all data to the vector
                //     write_vec.push((var, w, lock));
                // }
                
                // // We need to check for consistency and
                // // take a write lock.
                // &ReadWrite(ref original,ref w) => {
                //     // take write lock
                //     let lock = var.value.write().unwrap();

                //     if !same_address(&lock, original) {
                //         return false;
                //     }
                //     // add all data to the vector
                //     write_vec.push((var, w, lock));
                // }
                // // Nothing to do. ReadObsolete is only needed for blocking, not
                // // for consistency checks.
                // &ReadObsolete(_) => { }
                // // Take read lock and check for consistency.
                // &Read(ref original) => {
                //     // take a read lock
                //     let lock = var.value.read().unwrap();

                //     if !same_address(&lock, original) {
                //         return false;
                //     }

                //     read_vec.push(lock);
                // }
            }
        }

        // Second phase: write back and release

        // Release the reads first.
        //mem::drop(read_vec);

        for (var, value, mut lock) in write_vec {
            // commit value
            *lock = value.clone();
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        // commit succeded
        true
    }
}


fn arc_to_address<T: ?Sized>(arc: &Arc<T>) -> usize {
    &**arc as *const T as *const u32 as usize
}

fn same_address<T: ?Sized>(a: &Arc<T>, b: &Arc<T>) -> bool {
    arc_to_address(a) == arc_to_address(b)
}


/// Test same_address on a cloned Arc
#[test]
fn test_same_address_equal() {
    let t1 = Arc::new(42);
    let t2 = t1.clone();
    
    assert!(same_address(&t1, &t2));
}

/// Test same_address on differenc Arcs with same value
#[test]
fn test_same_address_different() {
    let t1 = Arc::new(42);
    let t2 = Arc::new(42);
    
    assert!(!same_address(&t1, &t2));
}

// #[test]
// fn test_read() {
//     let mut log = Transaction::new();
//     let var = TVar::new(vec![1, 2, 3, 4]);

//     // the variable can be read
//     assert_eq!(&*log.read(&var).unwrap(), &[1, 2, 3, 4]);
// }

// #[test]
// fn test_write_read() {
//     let mut log = Transaction::new();
//     let var = TVar::new(vec![1, 2]);

//     log.write(&var, vec![1, 2, 3, 4]).unwrap();

//     // consecutive reads get the updated version
//     assert_eq!(log.read(&var).unwrap(), [1, 2, 3, 4]);

//     // the original value is still preserved
//     assert_eq!(var.read_atomic(), [1, 2]);
// }

#[test]
fn test_transaction_simple() {
    let x = Transaction::with(|_| Ok(42));
    assert_eq!(x, 42);
}

#[test]
fn test_transaction_read() {
    let read = TVar::new(42);

    let x = Transaction::with(|trans| {
        read.read(trans)
    });

    assert_eq!(x, 42);
}

#[test]
fn test_transaction_write() {
    let write = TVar::new(42);

    Transaction::with(|trans| {
        write.write(trans, 0)
    });

    assert_eq!(write.read_atomic(), 0);
}

#[test]
fn test_transaction_copy() {
    let read = TVar::new(42);
    let write = TVar::new(0);

    Transaction::with(|trans| {
        let r = try!(read.read(trans));
        write.write(trans, r)
    });

    assert_eq!(write.read_atomic(), 42);
}

