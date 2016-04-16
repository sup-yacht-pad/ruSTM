use std::marker::PhantomData;
use super::Transaction;


pub struct TVar<T> {
    _marker: PhantomData<T>,
    pub value: T,
}

impl<T> TVar<T>
{
    pub fn new(val: T) -> TVar<T> {
        TVar {
            _marker: PhantomData,
            value: val,
        }
    }

   	pub fn read(&mut self, transaction: &mut Transaction) -> StmResult<T> {
        transaction.read(&mut self)
    }

	pub fn write(&mut self, transaction: &mut Transaction, value: T) -> StmResult<()> {
	    transaction.write(&mut self, value)
	}

}