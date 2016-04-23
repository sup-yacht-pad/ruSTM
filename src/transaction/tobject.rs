use std::collections::HashMap;
use std::vec::Vec;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::cell::Cell;
use std::mem; // XXX what are you doing with your life?

use variable::{TVar, Address};
use res::{StmResult, StmError};

static GLOBAL_SEQ_LOCK: AtomicUsize = ATOMIC_USIZE_INIT;

// XXX what was the point of this besides acting tough?
struct Value(pub usize);

pub struct Transaction {
    snapshot : usize,
    write_set: HashMap<Address, Value>,
    read_set: Vec<(Address, Value)>,
}

impl Transaction {
    // XXX what is this ss??? wouldn't it make more sense to pull it from the global directly?
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

    pub fn read<T>(&mut self, var: &TVar<T>) -> StmResult<T> {
        match self.write_set.get(&var.get_addr()) {
            // XXX UHHHH something is very wrong; what was the point of keeping
            // PhantomData<T> around if we're just going to do this?
            // RELOOK
            // xref: variable.rs:6
            Some(&Value(val)) => { return Ok(mem::transmute(val)); }
            None => {}
        }
        let mut val = (*var).value;
        while self.snapshot != GLOBAL_SEQ_LOCK.load(Ordering::SeqCst) {
            match self.validate() {
                None => { return Err(StmError::Retry); }
                Some(ss) => {
                    self.snapshot = ss;
                    val = (*var).value;
                }
            }
        }
        self.read_set.push((var.get_addr(), Value(val)));
        // XXX same fucking comment
        Ok(mem::transmute(val))
    }

    pub fn write<T>(&mut self, var: &mut TVar<T>, value: T) -> StmResult<()> {
        self.write_set.insert(var.get_addr(), Value(mem::transmute(value)));
        Ok(())

    }

    // XXX what is this for?
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
            for (Address(addr), Value(val)) in self.read_set {
                // XXX wait what this is WRONG you idiot
                // you're comparing a memory address to the value
                // that might be stored there... omg...
                if addr != val {
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
        if self.write_set.is_empty() {
            return true;
        }
        while GLOBAL_SEQ_LOCK.compare_and_swap(self.snapshot, self.snapshot+1, Ordering::SeqCst) != self.snapshot {
            match self.validate() {
                None => { return false; } // XXX why is this bailing?
                Some(ss) => { self.snapshot = ss; }
            }
        }
        for (addr, val) in self.write_set.iter() {
            // XXX gotta cast this addr back to TVar! urgh
            (*addr).value = val;
        }
        // XXX is this really just a plain store? seems sketchy
        GLOBAL_SEQ_LOCK.store(self.snapshot + 2, Ordering::SeqCst);
        return true;
    }
}

