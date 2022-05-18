//! Adapters to/from other IO traits.
//!
//! To interoperate with other IO trait ecosystems, wrap a type in one of these
//! adapters.
//!
//! There's no separate adapters for Read/ReadBuf/Write traits. Instead, a single
//! adapter implements the right traits based on what the inner type implements.
//! This allows adapting a `Read+Write`, for example.

mod std_io;
use core::fmt::Debug;

pub use std_io::*;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
mod futures_io;
#[cfg(feature = "async")]
pub use futures_io::*;

#[cfg(all(feature = "async", feature = "tokio"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "async", feature = "tokio"))))]
mod tokio;
#[cfg(all(feature = "async", feature = "tokio"))]
pub use crate::adapters::tokio::*;

fn to_io_error<T: Debug>(err: T) -> std::io::Error {
    let kind = std::io::ErrorKind::Other;
    std::io::Error::new(kind, format!("{:?}", err))
}

impl crate::Error for std::io::Error {
    fn kind(&self) -> crate::ErrorKind {
        crate::ErrorKind::Other
    }
}
