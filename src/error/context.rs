use super::Result;

// Extension trait to add context to errors in a convenient way. This allows us to easily add additional information to errors as they propagate up the call stack, making it easier to debug issues when they occur. The context method takes a string message and attaches it to the error, while the with_context method takes a closure that generates the message, allowing for lazy evaluation of the context string only when an error occurs.

// The ErrorContext trait is implemented for both Result and Option types, allowing us to use the context methods on any operation that returns a Result or an Option. For Result, we map the error to include the context message. For Option, we convert it to a Result and use the context message if the option is None.
pub trait ErrorContext<T> {
    fn context<S: Into<String>>(self, msg: S) -> Result<T>;
    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

// Implement ErrorContext for Result types. This allows us to easily add context to any error that occurs in a Result, by providing a message that describes the context of the error. The context is added by mapping the error to include the provided message.
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

// Implement ErrorContext for Option types. This allows us to convert an Option into a Result, where if the Option is None, we can provide a context message that describes the error. This is useful for cases where we expect a value to be present, and if it is not, we want to return an error with a helpful message.
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
