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

impl<T: ?Sized + Read> Read for &mut T {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        T::read(self, buf)
    }
}

impl<T: ?Sized + BufRead> BufRead for &mut T {
    type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
    where
        Self: 'a;

    fn fill_buf<'a>(&'a mut self) -> Self::FillBufFuture<'a> {
        T::fill_buf(self)
    }

    fn consume(&mut self, amt: usize) {
        T::consume(self, amt)
    }
}

impl<T: ?Sized + Write> Write for &mut T {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        T::write(self, buf)
    }

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        T::flush(self)
    }
}

/// Read is implemented for `&[u8]` by copying from the slice.
///
/// Note that reading updates the slice to point to the yet unread part.
/// The slice will be empty when EOF is reached.
impl Read for &[u8] {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        async move {
            let amt = core::cmp::min(buf.len(), self.len());
            let (a, b) = self.split_at(amt);

            // First check if the amount of bytes we want to read is small:
            // `copy_from_slice` will generally expand to a call to `memcpy`, and
            // for a single byte the overhead is significant.
            if amt == 1 {
                buf[0] = a[0];
            } else {
                buf[..amt].copy_from_slice(a);
            }

            *self = b;
            Ok(amt)
        }
    }
}

impl BufRead for &[u8] {
    type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn fill_buf<'a>(&'a mut self) -> Self::FillBufFuture<'a> {
        async move { Ok(*self) }
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

/// Write is implemented for `&mut [u8]` by copying into the slice, overwriting
/// its data.
///
/// Note that writing updates the slice to point to the yet unwritten part.
/// The slice will be empty when it has been completely overwritten.
///
/// If the number of bytes to be written exceeds the size of the slice, write operations will
/// return short writes: ultimately, `Ok(0)`; in this situation, `write_all` returns an error of
/// kind `ErrorKind::WriteZero`.
impl Write for &mut [u8] {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        async move {
            let amt = core::cmp::min(buf.len(), self.len());
            let (a, b) = core::mem::replace(self, &mut []).split_at_mut(amt);
            a.copy_from_slice(&buf[..amt]);
            *self = b;
            Ok(amt)
        }
    }

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        async move { Ok(()) }
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + Read> Read for alloc::boxed::Box<T> {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        T::read(self, buf)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + BufRead> BufRead for alloc::boxed::Box<T> {
    type FillBufFuture<'a> = impl Future<Output = Result<&'a [u8], Self::Error>>
    where
        Self: 'a;

    fn fill_buf<'a>(&'a mut self) -> Self::FillBufFuture<'a> {
        T::fill_buf(self)
    }

    fn consume(&mut self, amt: usize) {
        T::consume(self, amt)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + Write> Write for alloc::boxed::Box<T> {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        T::write(self, buf)
    }

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        T::flush(self)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl Write for alloc::vec::Vec<u8> {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        async move {
            self.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    #[inline]
    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        async move { Ok(()) }
    }
}
