use std::collections::HashMap;
use std::vec::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use variable::{TVari32, VarControlBlocki32};
use res::{StmResult, StmError};

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct Transaction {
    snapshot : usize,
    write_set: HashMap<usize, i32>,
    read_set: Vec<(usize, i32)>,
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

    pub fn readi32(&mut self, var: &TVari32) -> StmResult<i32> {
        let addr = var.get_addr();
        println!("this is the fucking address: {0:x}", addr);
        match self.write_set.get(&addr) {
            Some(&value) => { return Ok(value); }
            None => {}
        }
        let mut val = var.value;
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            match self.validate() {
                None => { return Err(StmError::Retry); }
                Some(ss) => {
                    self.snapshot = ss;
                    val = var.value;
                }
            }
        }
        self.read_set.push((addr, val));
        Ok(val)
    }

    pub fn writei32(&mut self, var: &TVari32, value: i32) -> StmResult<()> {
        let addr = var.get_addr();
        println!("this is the cheebye address: {0:x}", addr);
        self.write_set.insert(addr, value);
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
            for &(addr, val) in &self.read_set {
                let tvar = unsafe {*(addr as *mut TVari32)};
                let cur_val = tvar.value;
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
        for (&addr, &val) in &self.write_set {
            let mut tvar = unsafe {*(addr as *mut TVari32)};
            tvar.commit(val.clone());
            tvar.value = val.clone();
            println!("committing to address {0:x}", addr);
            println!("the new value is: {}", tvar.read_atomic());
        }
        for (&addr, &val) in &self.write_set {
            let mut tvar = unsafe {*(addr as *mut TVari32)};
            println!("verifying commit to address {0:x}", addr);
            println!("the new value is: {}", tvar.read_atomic());
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        true
    }
}

