//! Inkjot: the Twee 3 dialect behind Inklinks.
//!
//! This crate is the compiler's front end. Today it offers one operation,
//! [`check`], which reads an Inkjot source file and reports what is wrong with it.

pub mod check;
pub mod diag;
pub mod expr;
pub mod json;
pub mod resolve;
pub mod twee;

pub use check::{check, Report, Stats, VERSION};
pub use diag::{render, Diag, Severity};
