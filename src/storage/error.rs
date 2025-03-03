
use std::fmt::Display;

use oci_spec::OciSpecError;
use thiserror::Error;



pub type Result<T> = std::result::Result<T,StorageError>;

#[derive(Debug,Error)]
pub enum StorageError {
    
    OperatorError(opendal::Error),

    OciSpec(#[from]OciSpecError),
    
    SerdeParse(#[from] serde_json::Error),

    ContenNotFound,
    
    RangeIsNotStatisfied
}

impl Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}

impl From<opendal::Error> for StorageError{
    fn from(value: opendal::Error) -> Self {
        match value.kind() {
            opendal::ErrorKind::NotFound => StorageError::ContenNotFound,
            opendal::ErrorKind::RangeNotSatisfied => StorageError::RangeIsNotStatisfied,
            _ => StorageError::OperatorError(value),
        }
    }
}
