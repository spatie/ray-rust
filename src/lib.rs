#![doc = include_str!("../README.md")]

mod client;
mod payload;
mod ray;

pub use client::{Client, Config};
pub use payload::{Frame, Origin, Payload};
pub use ray::Ray;
pub use serde_json::{Value, json};

#[doc(hidden)]
pub use serde_json::to_value;

/// A serialization, configuration, transport or execution-control failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A value could not be serialized or JSON could not be decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The Ray server could not be reached, or returned an HTTP error.
    #[error(transparent)]
    Transport(#[from] ureq::Error),
    /// A local file could not be read.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// XML could not be parsed.
    #[error("invalid XML: {0}")]
    Xml(String),
    /// An environment variable has an invalid value.
    #[error("invalid Ray configuration: {0}")]
    Config(String),
    /// Another thread panicked while modifying this client's state.
    #[error("Ray client state is poisoned")]
    StatePoisoned,
    /// The Stop execution button was pressed in Ray.
    #[error("stop execution requested in Ray")]
    StopExecutionRequested,
}

/// Send values to Ray, capturing the invocation's file and line number.
///
/// With no arguments this returns a fresh [`Ray`] handle. With arguments it
/// sends one payload per value and returns `Result<Ray, Error>`. Expressions
/// are evaluated once. Values must implement [`serde::Serialize`]. Transport
/// errors are ignored by default, as in PHP; use [`Config::throws`] to report them.
///
/// ```rust
/// # fn main() -> Result<(), spatie_ray::Error> {
/// use spatie_ray::ray;
/// ray!("Hello world", true)?.green()?.label("Example")?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! ray {
    () => { $crate::Ray::new() };
    (client: $client:expr, $($value:expr),+ $(,)?) => {{
        (|| -> ::std::result::Result<$crate::Ray, $crate::Error> {
            let values = [$($crate::to_value(&$value)?),+];
            $client.ray().send_many(&values)
        })()
    }};
    ($($value:expr),+ $(,)?) => {{
        (|| -> ::std::result::Result<$crate::Ray, $crate::Error> {
            let values = [$($crate::to_value(&$value)?),+];
            $crate::Ray::new().send_many(&values)
        })()
    }};
}

#[cfg(doctest)]
mod guide {
    include!(concat!(env!("OUT_DIR"), "/documentation.rs"));
}
