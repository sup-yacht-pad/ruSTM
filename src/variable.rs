use std::marker::PhantomData;
use transaction::Transaction;
use res::StmResult;
use std::sync::Arc;
use std::any::Any;
use std::cmp::Eq;

pub struct VarControlBlocki32 {
    pub value: i32,
}

impl VarControlBlocki32 {
    pub fn new(val: i32) -> VarControlBlocki32
    {
        let ctrl = VarControlBlocki32 {
            value: val,
        };
        ctrl
    }

    fn get_addr(&mut self) -> usize {
        self as *mut VarControlBlocki32 as usize
    }
}

pub struct TVari32 {
    _marker: PhantomData<i32>,
    pub control_block: VarControlBlocki32,
}

impl TVari32
{
    pub fn new(val: i32) -> TVari32 {
        TVari32 {
            _marker: PhantomData,
            control_block: VarControlBlocki32::new(val),
        }
    }

   	pub fn read(&mut self, transaction: &mut Transaction) -> StmResult<i32> {
        transaction.readi32(self)
    }

	pub fn write(&mut self, transaction: &mut Transaction, value: i32) -> StmResult<()> {
	    transaction.writei32(self, value)
	}

    pub fn get_block_addr(&mut self) -> usize {
        self.control_block.get_addr()
    }
}

