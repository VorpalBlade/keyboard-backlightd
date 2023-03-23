use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub(crate) struct NoSuchFile {
    path: PathBuf,
}

impl Display for NoSuchFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Could not find: {:?}. Maybe --wait is too short (or there is a typo)?",
            self.path
        )
    }
}

impl Error for NoSuchFile {}

/// Wait for a file to show up
pub(crate) fn wait_for_file(path: &Path, timeout: Duration) -> Result<(), NoSuchFile> {
    let last_time = Instant::now() + timeout;
    while Instant::now() < last_time {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(NoSuchFile {
        path: path.to_path_buf(),
    })
}
