//! Async IO traits

use core::future::Future;

pub use crate::blocking::ReadExactError;

type ReadExactFuture<'a, T>
where
    T: Read + ?Sized + 'a,
= impl Future<Output = Result<(), ReadExactError<T::Error>>>;

/// Async reader.
///
/// Semantics are the same as [`std::io::Read`], check its documentation for details.
pub trait Read: crate::Io {
    /// Future returned by `read`.
    type ReadFuture<'a>: Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a>;

    /// Read the exact number of bytes required to fill `buf`.
    fn read_exact<'a>(&'a mut self, mut buf: &'a mut [u8]) -> ReadExactFuture<'a, Self> {
        async move {
            while !buf.is_empty() {
                match self.read(buf).await {
                    Ok(0) => break,
                    Ok(n) => buf = &mut buf[n..],
                    Err(e) => return Err(ReadExactError::Other(e)),
                }
            }
            if !buf.is_empty() {
                Err(ReadExactError::UnexpectedEof)
            } else {
                Ok(())
            }
        }
    }
}

/// Async buffered reader.
///
/// Semantics are the same as [`std::io::BufRead`], check its documentation for details.
pub trait BufRead: crate::Io {
    /// Future returned by `fill_buf`.
    type FillBufFuture<'a>: Future<Output = Result<&'a [u8], Self::Error>>
    where
        Self: 'a;

    /// Return the contents of the internal buffer, filling it with more data from the inner reader if it is empty.
    fn fill_buf<'a>(&'a mut self) -> Self::FillBufFuture<'a>;

    /// Tell this buffer that `amt` bytes have been consumed from the buffer, so they should no longer be returned in calls to `fill_buf`.
    fn consume(&mut self, amt: usize);
}

type WriteAllFuture<'a, T>
where
    T: Write + ?Sized + 'a,
= impl Future<Output = Result<(), T::Error>>;

/// Async writer.
///
/// Semantics are the same as [`std::io::Write`], check its documentation for details.
pub trait Write: crate::Io {
    /// Future returned by `write`.
    type WriteFuture<'a>: Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    /// Write a buffer into this writer, returning how many bytes were written.
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a>;

    /// Future returned by `flush`.
    type FlushFuture<'a>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a>;

    /// Write an entire buffer into this writer.
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAllFuture<'a, Self> {
        async move {
            let mut buf = buf;
            while !buf.is_empty() {
                match self.write(buf).await {
                    Ok(0) => panic!("zero-length write."),
                    Ok(n) => buf = &buf[n..],
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }
    }
}
