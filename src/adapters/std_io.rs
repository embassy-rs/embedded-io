use super::to_io_error;

pub struct FromStd<T: ?Sized> {
    inner: T,
}

impl<T> FromStd<T> {
    pub fn new(inner: T) -> Self
    where
        T: Sized,
    {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: ?Sized> FromStd<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: ?Sized> crate::Io for FromStd<T> {
    type Error = std::io::Error;
}

impl<T: std::io::Read + ?Sized> crate::blocking::Read for FromStd<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.inner.read(buf)
    }
}

impl<T: std::io::Write + ?Sized> crate::blocking::Write for FromStd<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.inner.flush()
    }
}

pub struct ToStd<T: ?Sized> {
    inner: T,
}

impl<T> ToStd<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: ?Sized> ToStd<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: crate::blocking::Read + ?Sized> std::io::Read for ToStd<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.inner.read(buf).map_err(to_io_error)
    }
}

impl<T: crate::blocking::Write + ?Sized> std::io::Write for ToStd<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.inner.write(buf).map_err(to_io_error)
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.inner.flush().map_err(to_io_error)
    }
}
