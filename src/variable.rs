use std::marker::PhantomData;
use transaction::Transaction;
use res::StmResult;

pub struct TVar<T> {
    _marker: PhantomData<T>, // do we really care about this? xref: tobject.rs:56
    // XXX "better" plan: either bound this with a trait for types that can be
    // turned into usize, or make functions take Into<usize> (figure out if this is possible)
    pub value: usize,
}

#[derive(PartialEq, Eq, Hash)]
pub struct Address(pub usize);

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

    pub fn get_addr(&mut self) -> Address {
        Address(self as *mut TVar<T> as usize)
    }
}

