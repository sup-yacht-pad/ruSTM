use std::marker::PhantomData;
use transaction::Transaction;
use res::StmResult;
use std::sync::Arc;
use std::cmp;

#[derive(Clone)]
pub struct VarControlBlocki32 {
    pub value: i32,
}

impl VarControlBlocki32 {
    pub fn new(val: i32) -> Arc<VarControlBlocki32>
    {
        let ctrl = VarControlBlocki32 {
            value: val,
        };
        Arc::new(ctrl)
    }

    fn get_addr(&self) -> usize {
        self as *const VarControlBlocki32 as usize
    }

    pub fn commit(&mut self, val: i32) -> () {
        self.value = val;
        ()
    }
}

impl PartialEq for VarControlBlocki32 {
    fn eq(&self, other: &Self) -> bool {
        self.get_addr() == other.get_addr()
    }
}

impl Eq for VarControlBlocki32 {}

impl Ord for VarControlBlocki32 {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get_addr().cmp(&other.get_addr())
    }
}

impl PartialOrd for VarControlBlocki32 {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct TVari32 {
    _marker: PhantomData<i32>,
    pub control_block: Arc<VarControlBlocki32>,
}

impl TVari32
{
    pub fn new(val: i32) -> TVari32 {
        TVari32 {
            _marker: PhantomData,
            control_block: VarControlBlocki32::new(val),
        }
    }

   	pub fn read(&self, transaction: &mut Transaction) -> StmResult<i32> {
        transaction.readi32(self)
    }

	pub fn write(&self, transaction: &mut Transaction, value: i32) -> StmResult<()> {
	    transaction.writei32(self, value)
	}

    pub fn control_block(&self) -> &Arc<VarControlBlocki32> {
        &self.control_block
    }

    pub fn read_atomic(&self) -> i32 {
        self.control_block.value
    }
}

