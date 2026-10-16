//! BenchMesh generic instrument driver — the interpreter of `devices/*.yaml`.
//!
//! This crate is I/O-free: framing, parsing, expressions, wire rendering and class status
//! are pure functions, so they are tested against the recorded wire captures. The engine
//! that owns a transport and schedules polls builds on top of these pieces.

pub mod expr;
pub mod framing;
pub mod parse;
pub mod pyfmt;
pub mod si;
pub mod status;
pub mod wire;

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Reply could not be framed (malformed header, data where none was expected).
    Framing(String),
    /// No complete reply before the timeout; `partial` holds what did arrive.
    Timeout { partial: Vec<u8> },
    /// Reply framed but did not match the parse pipeline.
    Parse(String),
    /// CEL evaluation failed.
    Expr(String),
    /// Caller supplied an invalid value (maps to HTTP 4xx).
    BadArgument(String),
    /// The definition itself is inconsistent (should have been caught by validation).
    Definition(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Framing(m) => write!(f, "framing: {m}"),
            Error::Timeout { partial } => write!(f, "timeout ({} bytes received)", partial.len()),
            Error::Parse(m) => write!(f, "parse: {m}"),
            Error::Expr(m) => write!(f, "expression: {m}"),
            Error::BadArgument(m) => write!(f, "bad argument: {m}"),
            Error::Definition(m) => write!(f, "definition: {m}"),
        }
    }
}

impl std::error::Error for Error {}
