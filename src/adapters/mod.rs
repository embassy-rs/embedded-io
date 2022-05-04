mod std_io;
use core::fmt::Debug;

pub use std_io::*;

#[cfg(feature = "async")]
mod futures_io;
#[cfg(feature = "async")]
pub use futures_io::*;

fn to_io_error<T: Debug>(err: T) -> std::io::Error {
    let kind = std::io::ErrorKind::Other;
    std::io::Error::new(kind, format!("{:?}", err))
}

impl crate::Error for std::io::Error {
    fn kind(&self) -> crate::ErrorKind {
        crate::ErrorKind::Other
    }
}
