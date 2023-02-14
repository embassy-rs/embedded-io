//! Async IO traits

pub use crate::blocking::ReadExactError;

///
/// Semantics are the same as [`std::io::Read`], check its documentation for details.
pub trait Read: crate::Io {
    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Read the exact number of bytes required to fill `buf`.
    async fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), ReadExactError<Self::Error>> {
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

/// Async buffered reader.
///
/// Semantics are the same as [`std::io::BufRead`], check its documentation for details.
pub trait BufRead: crate::Io {
    /// Return the contents of the internal buffer, filling it with more data from the inner reader if it is empty.
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error>;

    /// Tell this buffer that `amt` bytes have been consumed from the buffer, so they should no longer be returned in calls to `fill_buf`.
    fn consume(&mut self, amt: usize);
}

/// Async direct reader
pub trait DirectRead: crate::Io {
    /// The read handle type for this reader
    type Handle<'m>: DirectReadHandle<'m>;

    /// Read the next portion from this source.
    async fn read<'m>(&'m mut self) -> Result<Self::Handle<'m>, Self::Error>;
}

/// A direct read handle.
///
/// The buffer is returned to the source when the handle is dropped.
pub trait DirectReadHandle<'m> {
    /// Get the data slice.
    ///
    /// The entire data slice must be consumed.
    fn as_slice(&self) -> &[u8];

    /// Get whether the source has completed.
    ///
    /// There should be no more calls to [`DirectRead::read()`] after this.
    fn is_completed(&self) -> bool {
        self.as_slice().is_empty()
    }
}

impl<'m> DirectReadHandle<'m> for &'m[u8] {
    fn as_slice(&self) -> &[u8] {
        self
    }
}

/// An unbuffered [`Read`] wrapper for [`DirectRead`].
pub struct UnbufferedRead<T>
where
    T: crate::Io,
{
    source: T,
    is_completed: bool,
}

impl<T> UnbufferedRead<T>
where
    T: crate::Io,
{
    /// Create a new unbuffered wrapper for [`DirectRead`] implementing [`Read`].
    pub fn new(source: T) -> Self {
        Self {
            source,
            is_completed: false,
        }
    }
}

/// An unbuffered read error
#[derive(Debug)]
pub enum UnbufferedReadError<T: crate::Error> {
    /// The provided read buffer is too small to contain the entire slice returned by [`DirectRead::read()`].
    BufferTooSmall,
    /// Underlying i/o error
    Io(T),
}

impl<T> From<T> for UnbufferedReadError<T>
where
    T: crate::Error,
{
    fn from(value: T) -> Self {
        UnbufferedReadError::Io(value)
    }
}

impl<T: crate::Error> crate::Error for UnbufferedReadError<T> {
    fn kind(&self) -> crate::ErrorKind {
        match self {
            UnbufferedReadError::BufferTooSmall => crate::ErrorKind::Other,
            UnbufferedReadError::Io(other) => other.kind(),
        }
    }
}

impl<T> crate::Io for UnbufferedRead<T>
where
    T: crate::Io,
{
    type Error = UnbufferedReadError<T::Error>;
}

impl<T> Read for UnbufferedRead<T>
where
    T: DirectRead,
{
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.is_completed {
            return Ok(0);
        }

        loop {
            let handle = self.source.read().await?;

            // We cannot return empty slices as that would signify completion.
            if !handle.as_slice().is_empty() || handle.is_completed() {
                self.is_completed = handle.is_completed();

                let slice = handle.as_slice();
                if buf.len() >= slice.len() {
                    let len = core::cmp::min(slice.len(), buf.len());
                    buf[..len].copy_from_slice(&slice[..len]);
                    return Ok(len);
                } else {
                    return Err(UnbufferedReadError::BufferTooSmall);
                }
            }
        }
    }
}

/// Async writer.
///
/// Semantics are the same as [`std::io::Write`], check its documentation for details.
pub trait Write: crate::Io {
    /// Write a buffer into this writer, returning how many bytes were written.
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Write an entire buffer into this writer.
    async fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
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

/// Async seek within streams.
///
/// Semantics are the same as [`std::io::Seek`], check its documentation for details.
pub trait Seek: crate::Io {
    /// Seek to an offset, in bytes, in a stream.
    async fn seek(&mut self, pos: crate::SeekFrom) -> Result<u64, Self::Error>;

    /// Rewind to the beginning of a stream.
    async fn rewind(&mut self) -> Result<(), Self::Error> {
        self.seek(crate::SeekFrom::Start(0)).await?;
        Ok(())
    }

    /// Returns the current seek position from the start of the stream.
    async fn stream_position(&mut self) -> Result<u64, Self::Error> {
        self.seek(crate::SeekFrom::Current(0)).await
    }
}

impl<T: ?Sized + Read> Read for &mut T {
    #[inline]
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf).await
    }
}

impl<T: ?Sized + BufRead> BufRead for &mut T {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        T::fill_buf(self).await
    }

    fn consume(&mut self, amt: usize) {
        T::consume(self, amt)
    }
}

impl<T: ?Sized + Write> Write for &mut T {
    #[inline]
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        T::write(self, buf).await
    }

    #[inline]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        T::flush(self).await
    }
}

impl<T: ?Sized + Seek> Seek for &mut T {
    #[inline]
    async fn seek(&mut self, pos: crate::SeekFrom) -> Result<u64, Self::Error> {
        T::seek(self, pos).await
    }
}

/// Read is implemented for `&[u8]` by copying from the slice.
///
/// Note that reading updates the slice to point to the yet unread part.
/// The slice will be empty when EOF is reached.
impl Read for &[u8] {
    #[inline]
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
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

impl BufRead for &[u8] {
    #[inline]
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        Ok(*self)
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

impl DirectRead for &[u8] {
    type Handle<'m> = &'m[u8];

    #[inline]
    async fn read<'m>(&'m mut self) -> Result<Self::Handle<'m>, Self::Error> {
        Ok(self)
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
    #[inline]
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let amt = core::cmp::min(buf.len(), self.len());
        let (a, b) = core::mem::replace(self, &mut []).split_at_mut(amt);
        a.copy_from_slice(&buf[..amt]);
        *self = b;
        Ok(amt)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + Read> Read for alloc::boxed::Box<T> {
    #[inline]
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, buf).await
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + BufRead> BufRead for alloc::boxed::Box<T> {
    #[inline]
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        T::fill_buf(self).await
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        T::consume(self, amt)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + Write> Write for alloc::boxed::Box<T> {
    #[inline]
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        T::write(self, buf).await
    }

    #[inline]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        T::flush(self).await
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl<T: ?Sized + Seek> Seek for alloc::boxed::Box<T> {
    #[inline]
    async fn seek(&mut self, pos: crate::SeekFrom) -> Result<u64, Self::Error> {
        T::seek(self, pos).await
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "alloc"))))]
impl Write for alloc::vec::Vec<u8> {
    #[inline]
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
}
