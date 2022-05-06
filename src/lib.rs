#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "async",
    feature(generic_associated_types, type_alias_impl_trait)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

// mod fmt MUST go first, so that others see its macros.
mod fmt;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod async_;
pub mod blocking;

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod adapters;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
/// Possible kinds of errors.
pub enum ErrorKind {
    /// Unspecified error kind.
    Other,
}

/// Error trait.
///
/// This trait allows generic code to do limited inspecting of errors,
/// to react differently to different kinds.
pub trait Error: core::fmt::Debug {
    /// Get the kind of this error.
    fn kind(&self) -> ErrorKind;
}

/// Base trait for all IO traits.
///
/// All IO operations of all traits return the error defined in thsi trait.
///
/// Having a shared trait instead of having every trait define its own
/// `io::Error` enforces all impls on the same type use the same error.
/// This is very convenient when writing generic code, it means you have to
/// handle a single error type `T::Error`, instead of `<T as Read>::Error`, `<T as Write>::Error`
/// which might be different types.
pub trait Io {
    /// Error type of all the IO operations on this type.
    type Error: Error;
}

impl Error for core::convert::Infallible {
    fn kind(&self) -> ErrorKind {
        match *self {}
    }
}

impl Error for ErrorKind {
    fn kind(&self) -> ErrorKind {
        *self
    }
}
