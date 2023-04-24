//! Abstraction for LED in /sys

use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;

use crate::state::State;

const BRIGHTNESS: &str = "brightness";
const MAX_BRIGHTNESS: &str = "max_brightness";
const BRIGHTNESS_HW_CHANGED: &str = "brightness_hw_changed";

/// Helper to read an integer from a file
fn read_int(f: &mut File) -> anyhow::Result<u32> {
    let mut buf = String::new();
    f.rewind()?;
    f.read_to_string(&mut buf)?;
    Ok(buf.trim_end_matches('\n').parse()?)
}

/// Helper to read an integer from a path.
fn read_int_path(p: &Path) -> anyhow::Result<u32> {
    read_int(
        &mut OpenOptions::new()
            .read(true)
            .open(p)
            .with_context(|| format!("Failed to open {p:?}"))?,
    )
    .with_context(|| format!("Failed to parse integer from {p:?}"))
}

/// Get the path to monitor for HW changes. Not all LEDs support this.
fn monitor_path(mut led_path: PathBuf) -> anyhow::Result<Option<PathBuf>> {
    led_path.push(BRIGHTNESS_HW_CHANGED);
    if led_path
        .try_exists()
        .with_context(|| format!("Failed to check for LED monitoring path {led_path:?}"))?
    {
        Ok(Some(led_path))
    } else {
        Ok(None)
    }
}

#[derive(Debug)]
pub(crate) struct Led {
    /// Path to LED
    path: PathBuf,
    brightness_file: File,
    /// Optional hardware monitoring path
    hw_monitor_path: Option<PathBuf>,
}

impl Led {
    /// Create a new LED wrapper.
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let mut p = path.clone();
        p.push(BRIGHTNESS);
        let hw_monitor_path = monitor_path(path.clone())?;
        Ok(Self {
            path,
            brightness_file: OpenOptions::new()
                .read(true)
                .write(true)
                .open(&p)
                .with_context(|| format!("Failed to open {p:?}"))?,
            hw_monitor_path,
        })
    }

    /// Get the current brightness
    pub fn brightness(&mut self) -> anyhow::Result<u32> {
        read_int(&mut self.brightness_file).context("Failed to read brightness")
    }

    /// Get the max brightness supported
    #[allow(unused)]
    pub fn max_brightness(&self) -> anyhow::Result<u32> {
        let mut p = self.path.clone();
        p.push(MAX_BRIGHTNESS);
        read_int_path(p.as_path())
    }

    /// Set the current brightness
    pub fn set_brightness(&mut self, brightness: u32, state: &mut State) -> anyhow::Result<()> {
        let mut p = self.path.clone();
        p.push(BRIGHTNESS);

        // If hardware monitoring is not supported, try reading the previous value.
        if self.hw_monitor_path.is_none() && brightness == 0 {
            let old_brightness = read_int(&mut self.brightness_file)?;
            if old_brightness > 0 {
                state.requested_brightness = old_brightness;
            }
        }
        self.brightness_file.rewind()?;
        write!(self.brightness_file, "{brightness}")?;
        Ok(())
    }

    /// Get the path to monitor for HW changes. Not all LEDs support this.
    pub fn monitor_path(&self) -> Option<&Path> {
        return self.hw_monitor_path.as_deref();
    }
}
