use std::collections::HashMap;
use std::collections::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::cell::Cell;

use super::variable::{TVar};
use super::res::*;

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct Transaction {
    usize: snapshot
    writeSet: HashMap<&mut TVar<T>, u32>
    readSet: Vec<(&mut TVar<T>, u32)>
}

impl Transaction {
    fn new(ss: usize) -> Transaction {
        Transaction {
            snapShot: ss
            writeSet: BTreeMap::new()
            readSet: Vec::new()
        }
    }

    pub fn run<T, F>(f: F) -> T 
    where F: Fn(&mut Transaction) -> StmResult<T>,
    {
        let mut ss = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
        while ((ss & 1) != 0) {
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
            Some &val => return Ok(val)
            None => {}
        };
        let mut val = (*var).value;
        while (self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst)) {
            self.snapshot = self.validate()
            val = (*var).value;
            if (self.snapshot < 0) {
                return Err(Retry);
            }
        }
        self.readset.push((addr, val));
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

    fn validate(&mut self) -> <usize> {
        loop {
            let mut usize time = GLOBAL_SEQ_LOCK.load(Ordering::SeqCst);
            if ((time & 1) != 0) {
                continue;
            }
            for (addr, val) in self.readSet {
                if ((*addr).value != val) {
                    return -1;
                }
            }
            if (time == lock.load(Ordering::SeqCst)) {
                return time;
            }
        }
    }

    fn commit(&mut self) -> bool {
        if (self.writeSet.len() == 0) {
            return true;
        }
        while (GLOBAL_SEQ_LOCK.compare_and_swap.(self.snapshot, self.snapshot +  1, Ordering::SeqCst) != self.snapshot) {
            self.snapshot = self.validate()
            if (self.snapshot < 0) {
                return false;
            }
        }
        for (addr, val) in self.writeSet.iter() {
            (*addr).value = val;
        }
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        return true;
    }
}