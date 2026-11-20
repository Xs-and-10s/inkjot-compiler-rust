//! Inkjot: the Twee 3 dialect behind Inklinks.
//!
//! Two operations: [`check`] reads an Inkjot source file and reports what is
//! wrong with it; [`build`] compiles a file that passes into its JSON IR.
//! [`engine`] plays the IR, for a program that hosts Inklinks.

pub mod check;
pub mod diag;
pub mod engine;
pub mod expr;
pub mod ir;
pub mod json;
pub mod migrate;
pub mod resolve;
pub mod run;
mod syntax;
pub mod text;
pub mod twee;

pub use check::{check, Report, Stats, VERSION};
pub use diag::{render, Diag, Severity};
pub use ir::build;
