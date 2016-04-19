use std::marker::PhantomData;
use transaction::Transaction;
use res::StmResult;

pub struct TVar<T> {
    _marker: PhantomData<T>, // do we really care about this?
    pub value: usize,
}

impl<T> TVar<T>
{
    pub fn new(val: usize) -> TVar<T> {
        TVar {
            _marker: PhantomData,
            value: val,
        }
    }

   	pub fn read(&mut self, transaction: &mut Transaction) -> StmResult<T> {
        transaction.read(self)
    }

	pub fn write(&mut self, transaction: &mut Transaction, value: T) -> StmResult<()> {
	    transaction.write(self, value)
	}

    pub fn get_addr(&self) -> usize {
        self._marker as *const PhantomData<T>
    }
}

