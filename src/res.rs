pub enum StmError {
    Fail,
    Retry,
}

pub type StmResult<T> = Result<T, StmError>;

