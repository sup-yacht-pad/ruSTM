
use std::sync::{Arc, RwLock};
use std::mem;
use std::cmp;
use std::any::Any;
use std::marker::PhantomData;

use super::result::*;
use super::Transaction;

pub struct VarControlBlock {
    pub value: RwLock<Arc<Any + Send + Sync>>,
}

impl VarControlBlock {
    pub fn new<T>(val: T) -> Arc<VarControlBlock>
        where T: Any + Sync + Send
    {
        let ctrl = VarControlBlock {
            value: RwLock::new(Arc::new(val)),
        };
        Arc::new(ctrl)
    }

    fn get_address(&self) -> usize {
        self as *const VarControlBlock as usize
    }
}

impl PartialEq for VarControlBlock {
    fn eq(&self, other: &Self) -> bool {
        self.get_address() == other.get_address()
    }
}

impl Eq for VarControlBlock {}

impl Ord for VarControlBlock {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get_address().cmp(&other.get_address())
    }
}

impl PartialOrd for VarControlBlock {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct TVar<T> {
    control_block: Arc<VarControlBlock>,
    _marker: PhantomData<T>,
}

impl<T> TVar<T>
    where T: Any + Sync + Send + Clone
{
    pub fn new(val: T) -> TVar<T> {
        TVar {
            control_block: VarControlBlock::new(val),
            _marker: PhantomData,
        }
    }

    pub fn read_atomic(&self) -> T {
        let val = self.read_ref_atomic();

        (&*val as &Any)
            .downcast_ref::<T>()
            .expect("wrong type in Var<T>")
            .clone()
    }

    pub fn read_ref_atomic(&self) -> Arc<Any + Send + Sync> {
        self.control_block
            .value
            .read()
            .unwrap()
            .clone()
    }

    pub fn read(&self, transaction: &mut Transaction) -> StmResult<T> {
        transaction.read(&self)
    }

    pub fn write(&self, transaction: &mut Transaction, value: T) -> StmResult<()> {
        transaction.write(&self, value)
    }
    
    pub fn control_block(&self) -> &Arc<VarControlBlock> {
        &self.control_block
    }
}
