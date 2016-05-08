pub mod control_block;
pub mod log_var;

use std::collections::BTreeMap;
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

    pub fn run<T, F>(f: F) -> T 
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
            Some(entry) => {let value = entry.clone();
                                    return Ok(Transaction::downcast(value));}
            None => { }
        }
        let mut value = var.read_ref_atomic();
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            match self.validate() {
                None => { return Err(Retry); }
                Some(ss) => {
                    self.snapshot = ss;
                    value = var.read_ref_atomic();
                }
            }
        }
        let ctrl = var.control_block().clone();
        self.readvars.insert(ctrl, value.clone());
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
                        ref original => {
                        let lock = var.value.read().unwrap();
                        if !same_address(&lock, &original) {
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
        let mut write_vec = Vec::new();

        for (var, value) in &vars {
            match value {
                ref val => {
                    let lock = var.value.write().unwrap();
                    write_vec.push((val.clone(), lock));
                }
            }
        }
        for (value, mut lock) in write_vec {
            *lock = value.clone();
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        true
    }
}


fn arc_to_address<T: ?Sized>(arc: &Arc<T>) -> usize {
    &**arc as *const T as *const u32 as usize
}

fn same_address<T: ?Sized>(a: &Arc<T>, b: &Arc<T>) -> bool {
    arc_to_address(a) == arc_to_address(b)
}