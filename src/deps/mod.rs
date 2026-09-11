//! Narrow reimplementations of the observable behaviour that the Go build got
//! from its standard library and from third-party packages.
//!
//! Each module here exists because some *user-visible* output of direnv is
//! produced by Go code that Rust has no equivalent for: the exact wording of a
//! `filepath` result, the exact spelling of a duration, the exact bytes of a
//! JSON document. They reproduce that behaviour; they are not general-purpose
//! ports of the packages they are named after.

pub mod goerr;
pub mod goexpand;
pub mod gojson;
pub mod gopath;
pub mod goregexp;
pub mod gosemver;
pub mod gotemplate;
pub mod gotime;
