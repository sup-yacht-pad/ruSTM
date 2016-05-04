use std::marker::PhantomData;
use transaction::Transaction;
use res::StmResult;
use std::sync::Arc;
use std::any::Any;
use std::cmp::Eq;

pub struct VarControlBlock {
    pub value: Arc<Any>,
}

impl VarControlBlock {
    pub fn new<T>(val: T) -> Arc<VarControlBlock>
        where T: Any + Eq
    {
        let ctrl = VarControlBlock {
            value: Arc::new(val),
        };
        Arc::new(ctrl)
    }

    fn get_addr(&self) -> usize {
        self as *const VarControlBlock as usize
    }
}

#[derive(Clone)]
pub struct TVar<T> {
    _marker: PhantomData<T>,
    control_block: Arc<VarControlBlock>,
}

impl<T> TVar<T>
    where T: Any + Clone + Eq
{
    pub fn new(val: T) -> TVar<T> {
        TVar {
            _marker: PhantomData,
            control_block: VarControlBlock::new(val),
        }
    }

   	pub fn read(&self, transaction: &mut Transaction) -> StmResult<T> {
        transaction.read(self)
    }

	pub fn write(&self, transaction: &mut Transaction, value: T) -> StmResult<()> {
	    transaction.write(self, value)
	}

    pub fn get_addr(&mut self) -> usize {
        self as *mut TVar<T> as usize
    }

    pub fn get_block_addr(&mut self) -> usize {
        self.control_block.get_addr()
    }
}

