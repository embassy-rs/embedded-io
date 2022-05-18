//! Blocking IO traits

use core::fmt;

/// Error returned by [`Read::read_exact`]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReadExactError<E> {
    /// An EOF error was encountered before reading the exact amount of requested bytes.
    UnexpectedEof,
    /// Error returned by the inner Read.
    Other(E),
}

impl<E: fmt::Debug> fmt::Display for ReadExactError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug> std::error::Error for ReadExactError<E> {}

/// Error returned by [`Write::write_fmt`]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WriteFmtError<E> {
    /// An error was encountered while formatting.
    FmtError,
    /// Error returned by the inner Write.
    Other(E),
}

impl<E: fmt::Debug> fmt::Display for WriteFmtError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug> std::error::Error for WriteFmtError<E> {}

/// Blocking reader.
///
/// Semantics are the same as [`std::io::Read`], check its documentation for details.
pub trait Read: crate::Io {
    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Read the exact number of bytes required to fill `buf`.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
        while !buf.is_empty() {
            match self.read(buf) {
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

/// Blocking buffered reader.
///
/// Semantics are the same as [`std::io::BufRead`], check its documentation for details.
pub trait BufRead: crate::Io {
    /// Return the contents of the internal buffer, filling it with more data from the inner reader if it is empty.
    fn fill_buf(&mut self) -> Result<&[u8], Self::Error>;

    /// Tell this buffer that `amt` bytes have been consumed from the buffer, so they should no longer be returned in calls to `fill_buf`.
    fn consume(&mut self, amt: usize);
}

/// Blocking writer.
///
/// Semantics are the same as [`std::io::Write`], check its documentation for details.
pub trait Write: crate::Io {
    /// Write a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    fn flush(&mut self) -> Result<(), Self::Error>;

    /// Write an entire buffer into this writer.
    fn write_all(&mut self, mut buf: &[u8]) -> Result<(), Self::Error> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => panic!("zero-length write."),
                Ok(n) => buf = &buf[n..],
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Write a formatted string into this writer, returning any error encountered.
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<(), WriteFmtError<Self::Error>> {
        // Create a shim which translates a Write to a fmt::Write and saves
        // off I/O errors. instead of discarding them
        struct Adapter<'a, T: Write + ?Sized + 'a> {
            inner: &'a mut T,
            error: Result<(), T::Error>,
        }

        impl<T: Write + ?Sized> fmt::Write for Adapter<'_, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                match self.inner.write_all(s.as_bytes()) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        self.error = Err(e);
                        Err(fmt::Error)
                    }
                }
            }
        }

        let mut output = Adapter {
            inner: self,
            error: Ok(()),
        };
        match fmt::write(&mut output, fmt) {
            Ok(()) => Ok(()),
            Err(..) => match output.error {
                // check if the error came from the underlying `Write` or not
                Err(e) => Err(WriteFmtError::Other(e)),
                Ok(()) => Err(WriteFmtError::FmtError),
            },
        }
    }
}

impl<T: ?Sized + Read> Read for &mut T {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf)
    }
}

impl<T: ?Sized + Write> Write for &mut T {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        T::write(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        T::flush(self)
    }
}

#[cfg(feature = "alloc")]
impl<T: ?Sized + Read> Read for alloc::boxed::Box<T> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf)
    }
}

#[cfg(feature = "alloc")]
impl<T: ?Sized + Write> Write for alloc::boxed::Box<T> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        T::write(self, buf)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        T::flush(self)
    }
}

#[cfg(feature = "alloc")]
impl Write for alloc::vec::Vec<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
