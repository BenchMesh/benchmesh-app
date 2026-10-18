//! BenchMesh generic instrument driver — the interpreter of `devices/*.yaml`.
//!
//! Framing, parsing, expressions, wire rendering and class status are pure functions,
//! tested against the recorded wire captures. [`engine::Engine`] executes a definition
//! over a [`engine::Transport`]; [`actor::DeviceHandle`] runs one engine per thread.
//! Real transports (serial, USBTMC) live in `bm-transport`.

pub mod actor;
pub mod engine;
pub mod expr;
pub mod framing;
pub mod metrics;
pub mod modbus;
pub mod parse;
pub mod pyfmt;
pub mod replay;
pub mod si;
pub mod sim;
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
    /// A `requires` precondition is false (maps to HTTP 409 with the message).
    Refused(String),
    /// Unknown class / operation (maps to HTTP 404).
    NotFound(String),
    /// Identify reply matches no model of the definition.
    UnknownModel(String),
    /// The instrument answered but is not the configured one (e.g. another serial number).
    WrongInstrument(String),
    /// Transport failure: the link is lost.
    Io(String),
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
            Error::Refused(m) => write!(f, "refused: {m}"),
            Error::NotFound(m) => write!(f, "not found: {m}"),
            Error::UnknownModel(idn) => write!(f, "unknown model (identify reply {idn:?})"),
            Error::WrongInstrument(m) => write!(f, "wrong instrument: {m}"),
            Error::Io(m) => write!(f, "i/o: {m}"),
        }
    }
}

impl std::error::Error for Error {}
