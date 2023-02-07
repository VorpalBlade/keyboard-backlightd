//! Abstraction for LED in /sys

use std::{
    error::Error,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct Led {
    /// Path to LED
    path: PathBuf,
}

const BRIGHTNESS: &str = "brightness";
const MAX_BRIGHTNESS: &str = "max_brightness";
const BRIGHTNESS_HW_CHANGED: &str = "brightness_hw_changed";

/// Helper to read an integer from a path.
fn read_int(p: &Path) -> Result<u8, Box<dyn Error>> {
    let mut f = OpenOptions::new().read(true).open(p)?;
    let mut buf = String::new();
    f.read_to_string(&mut buf)?;
    Ok(buf.trim_end_matches('\n').parse()?)
}

impl Led {
    /// Create a new LED wrapper.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Get the current brightness
    pub fn brightness(&self) -> Result<u8, Box<dyn Error>> {
        let mut p = self.path.clone();
        p.push(BRIGHTNESS);
        read_int(p.as_path())
    }

    /// Get the max brightness supported
    #[allow(unused)]
    pub fn max_brightness(&self) -> Result<u8, Box<dyn Error>> {
        let mut p = self.path.clone();
        p.push(MAX_BRIGHTNESS);
        read_int(p.as_path())
    }

    /// Set the current brightness
    pub fn set_brightness(&mut self, brightness: u8) -> Result<(), Box<dyn Error>> {
        let mut p = self.path.clone();
        p.push(BRIGHTNESS);
        let mut f = OpenOptions::new().write(true).open(p)?;
        write!(f, "{brightness}")?;
        Ok(())
    }

    /// Get the path to monitor for HW changes. Not all LEDs support this.
    pub fn monitor_path(&self) -> Result<Option<PathBuf>, Box<dyn Error>> {
        let mut p = self.path.clone();
        p.push(BRIGHTNESS_HW_CHANGED);
        if p.try_exists()? {
            Ok(Some(p))
        } else {
            Ok(None)
        }
    }
}
