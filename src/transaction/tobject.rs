use std::collections::HashMap;
use std::vec::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::cell::Cell;

use variable::{TVar};
use res::{StmResult, StmError};

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

struct Address(pub usize);
struct Value(pub usize);

pub struct Transaction {
    snapshot : usize,
    writeSet: HashMap<Address, Value>,
    readSet: Vec<(Address, Value)>,
}

impl Transaction {
    // XXX what is this ss???
    fn new(ss: usize) -> Transaction {
        Transaction {
            snapShot: ss,
            writeSet: HashMap::new(),
            readSet: Vec::new(),
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

    pub fn read<T>(&mut self, var: &mut TVar<T>) -> StmResult<T> {
        match self.writeSet.get(&var) {
            Some(&val) => { return Ok(val) }
            None => {}
        };
        let mut val = (*var).value;
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            self.snapshot = self.validate();
            val = (*var).value;
            if self.snapshot < 0 {
                return Err(StmError::Retry);
            }
        }
        self.readset.push((self.get_addr(), val));
        Ok(val)
    }

    pub fn write<T>(&mut self, var: &mut TVar<T>, value: T) -> StmResult<()> {
        self.writeSet.insert(var, value);
        Ok(())

    }

    fn clear(&mut self) {
        self.readSet.clear();
        self.writeSet.clear();
    }

    fn validate(&self) -> Option<usize> {
        loop {
            let time = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
            if time & 1 != 0 {
                continue;
            }
            for (addr, val) in self.readSet {
                if (*addr).value != val {
                    // XXX this should handle aborts more... "gracefully"?
                    return None;
                }
            }
            if time == GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
                return Some(time);
            }
        }
    }

    fn commit(&mut self) -> bool {
        if self.writeSet.is_empty() {
            return true;
        }
        while GLOBAL_SEQ_LOCK.compare_and_swap(self.snapshot, self.snapshot+1, Ordering::SeqCst) != self.snapshot {
            self.snapshot = self.validate();
            if self.snapshot < 0 {
                // XXX I'm sorry, why is this bailing?
                return false;
            }
        }
        for (addr, val) in self.writeSet.iter() {
            (*addr).value = val;
        }
        // XXX is this really just a plain store? seems sketchy
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        return true;
    }
}

