use super::Result;

// Extension trait to add context to errors in a convenient way

//  we map the error to include the context message. For Option, we convert it to a Result and use the context message if the option is None.
pub trait ErrorContext<T> {
    fn context<S: Into<String>>(self, msg: S) -> Result<T>;
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
        self.map_err(|e| {
            super::PiramidError::Other(format!("{}: {}", msg.into(), e))
        })
    }

    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|e| {
            super::PiramidError::Other(format!("{}: {}", f().into(), e))
        })
    }
}

// allows us to convert an Option into a Result, where if the Option is None, we can provide a context message that describes the error. 
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
