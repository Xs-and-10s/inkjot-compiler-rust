//! Inkjot: the Twee 3 dialect behind Inklinks.
//!
//! Two operations: [`check`] reads an Inkjot source file and reports what is
//! wrong with it; [`build`] compiles a file that passes into its JSON IR.

pub mod check;
pub mod diag;
pub mod expr;
pub mod ir;
pub mod json;
pub mod resolve;
pub mod run;
mod syntax;
pub mod twee;

pub use check::{check, Report, Stats, VERSION};
pub use diag::{render, Diag, Severity};
pub use ir::build;
