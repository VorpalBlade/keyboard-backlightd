//! Error types

use std::num::ParseIntError;

use snafu::{prelude::*, Backtrace};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum KBError {
    #[snafu(display("Failed to open {path}: {source}"))]
    IoOpeningFile {
        path: String,
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("Failed to open {path}: Not found after --wait timeout"))]
    IoFileNotFound { path: String, backtrace: Backtrace },
    #[snafu(display("Unexpected IO error on {path}: {source}"))]
    GenericIoError {
        path: String,
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("IO error from LED module: {source}"))]
    LedError {
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("LED parse error: {source}"))]
    LedParse {
        source: ParseIntError,
        backtrace: Backtrace,
    },
    #[snafu(display("Inotify file monitoring error: {source}"))]
    Inotify {
        source: nix::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("Epoll polling error: {source}"))]
    Epoll {
        source: nix::Error,
        backtrace: Backtrace,
    },
}
