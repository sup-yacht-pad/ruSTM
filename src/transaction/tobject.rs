use std::collections::BTreeMap;
use std::vec::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::sync::Arc;

use variable::{TVari32, VarControlBlocki32};
use res::{StmResult, StmError};

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct Transaction {
    snapshot : usize,
    write_set: BTreeMap<Arc<VarControlBlocki32>, i32>,
    read_set: Vec<(Arc<VarControlBlocki32>, i32)>,
}

impl Transaction {
    fn new(ss: usize) -> Transaction {
        Transaction {
            snapshot: ss,
            write_set: BTreeMap::new(),
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

    pub fn readi32(&mut self, var: &TVari32) -> StmResult<i32> {
        let block = var.control_block().clone();
        match self.write_set.get(&block) {
            Some(&value) => { return Ok(value); }
            None => {}
        }
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
        self.read_set.push((block, val));
        Ok(val)
    }

    pub fn writei32(&mut self, var: &TVari32, value: i32) -> StmResult<()> {
        let block = var.control_block().clone();
        self.write_set.insert(block, value);
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
            let copy = self.read_set.clone();
            for (block, val) in copy {
                let cur_val = block.value;
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
        let copy = self.write_set.clone();
        for (mut block, val) in copy {
            let b = Arc::make_mut(&mut block);
            b.commit(val);
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        return true;
    }
}

