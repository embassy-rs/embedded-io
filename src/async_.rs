use core::future::Future;

pub use crate::blocking::ReadExactError;

type ReadExactFuture<'a, T>
where
    T: Read + ?Sized + 'a,
= impl Future<Output = Result<(), ReadExactError<T::Error>>>;

pub trait Read: crate::Io {
    type ReadFuture<'a>: Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a>;

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

pub trait BufRead: crate::Io {
    type FillBufFuture<'a>: Future<Output = Result<&'a [u8], Self::Error>>
    where
        Self: 'a;

    fn fill_buf<'a>(&'a mut self) -> Self::FillBufFuture<'a>;
    fn consume(&mut self, amt: usize);
}

type WriteAllFuture<'a, T>
where
    T: Write + ?Sized + 'a,
= impl Future<Output = Result<(), T::Error>>;

pub trait Write: crate::Io {
    type WriteFuture<'a>: Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a>;

    type FlushFuture<'a>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a>;

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
