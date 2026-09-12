//! Attaching a message to a failure.

use super::Result;

/// Attach a message to an error or to a None.
pub trait ErrorContext<T> {
    /// Convert to a [Result], prefixing any error with msg.
    fn context<S: Into<String>>(self, msg: S) -> Result<T>;
    /// Convert to a [Result], prefixing any error with the message f returns.
    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<S: Into<String>>(self, msg: S) -> Result<T> {
        self.map_err(|e| super::PiramidError::Other(format!("{}: {}", msg.into(), e)))
    }

    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|e| super::PiramidError::Other(format!("{}: {}", f().into(), e)))
    }
}

impl<T> ErrorContext<T> for Option<T> {
    fn context<S: Into<String>>(self, msg: S) -> Result<T> {
        self.ok_or_else(|| super::PiramidError::Other(msg.into()))
    }

    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.ok_or_else(|| super::PiramidError::Other(f().into()))
    }
}
