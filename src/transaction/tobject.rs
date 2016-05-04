use std::collections::HashMap;
use std::vec::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::cell::Cell;
use std::any::Any;
use std::sync::Arc;

use variable::{TVar, VarControlBlock};
use res::{StmResult, StmError};

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct Transaction {
    snapshot : usize,
    write_set: HashMap<usize, Arc<Any>>,
    read_set: Vec<(usize, Arc<Any>)>,
}

impl Transaction {
    fn new(ss: usize) -> Transaction {
        Transaction {
            snapshot: ss,
            write_set: HashMap::new(),
            read_set: Vec::new(),
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

    pub fn read<T: Any + Clone + Eq>(&mut self, var: &TVar<T>) -> StmResult<T> {
        let block_addr = var.get_block_addr();
        match self.write_set.get(&block_addr) {
            Some(&value) => { return Ok(Transaction::downcast(value)); }
            None => {}
        }
        let block = unsafe {*(block_addr as *const VarControlBlock)};
        let mut val = block.value;
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            match self.validate() {
                None => { return Err(StmError::Retry); }
                Some(ss) => {
                    self.snapshot = ss;
                    val = block.value;
                }
            }
        }
        self.read_set.push((block_addr, val));
        Ok(Transaction::downcast(val))
    }

    pub fn write<T: Any + Clone + Eq>(&mut self, var: &TVar<T>, value: T) -> StmResult<()> {
        let block_addr = var.get_block_addr();
        self.write_set.insert(block_addr, value);
        Ok(())
    }

    fn clear(&mut self) {
        self.read_set.clear();
        self.write_set.clear();
    }

    fn validate(&self) -> Option<usize> {
        loop {
            let time = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
            if time & 1 != 0 {
                continue;
            }
            for (addr, val) in self.read_set {
                let cur_val = unsafe {*(addr as *const VarControlBlock)}.value;
                if cur_val != val {
                    return None;
                }
            }
            if time == GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
                return Some(time);
            }
        }
    }

    fn commit(&mut self) -> bool {
        if self.write_set.is_empty() {
            return true;
        }
        while GLOBAL_SEQ_LOCK.compare_and_swap(self.snapshot, self.snapshot+1, Ordering::SeqCst) != self.snapshot {
            match self.validate() {
                None => { return false; }
                Some(ss) => { self.snapshot = ss; }
            }
        }
        for (addr, val) in self.write_set {
            let block = unsafe {*(addr as *const VarControlBlock)};
            block.value = val;
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        return true;
    }
}

