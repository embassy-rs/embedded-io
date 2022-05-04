#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "async",
    feature(generic_associated_types, type_alias_impl_trait)
)]

// mod fmt MUST go first, so that others see its macros.
mod fmt;

#[cfg(feature = "async")]
pub mod async_;
pub mod blocking;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ErrorKind {
    /// Unspecified error kind.
    Other,
}

pub trait Error {
    fn kind(&self) -> ErrorKind;
}

pub trait Io {
    type Error: Error;
}
