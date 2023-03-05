use crate::errors::KBError;

use snafu::GenerateImplicitData;
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// Wait for a file to show up
pub(crate) fn wait_for_file(path: &Path, timeout: Duration) -> Result<(), KBError> {
    let last_time = Instant::now() + timeout;
    while Instant::now() < last_time {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(KBError::IoFileNotFound {
        path: path.to_string_lossy().into(),
        backtrace: snafu::Backtrace::generate(),
    })
}
