// Functions return Ok(value) on success, Err(error) on failure.

use std::io;
use thiserror::Error;  

// Type alias: now we can write `Result<T>` instead of `Result<T, PiramidError>`
pub type Result<T> = std::result::Result<T, PiramidError>;

// thiserror's #[derive(Error)] generates Display and Error traits
// #[error("...")] defines the error message
// #[from] auto-implements From<X> for automatic conversion with ?
#[derive(Error, Debug)]
pub enum PiramidError {
    #[error("IO error: {0}")]           // {0} = first field
    Io(#[from] io::Error),              // #[from] lets ? auto-convert io::Error

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}
// Now when we do `file.read()?`, io::Error auto-converts to PiramidError::Io
