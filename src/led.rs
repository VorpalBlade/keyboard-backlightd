//! Abstraction for LED in /sys

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

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
    /// Path to LED (subdirectory in `/sys/class/leds`)
    path: PathBuf,
    /// File handle for brightness control file
    brightness_file: File,
    /// Optional hardware monitoring path
    hw_monitor_path: Option<PathBuf>,
    /// Most recently known brightness
    most_recent_brightness: Option<(Instant, u32)>,
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
            most_recent_brightness: None,
        })
    }

    /// Get the current brightness
    pub fn brightness(&mut self) -> anyhow::Result<u32> {
        let brightness = read_int(&mut self.brightness_file).context("Failed to read brightness");
        match &brightness {
            Ok(b) => {
                self.most_recent_brightness = Some((Instant::now(), *b));
            }
            Err(e) => {
                log::warn!("Failed to read brightness: {e}");
            }
        }
        brightness
    }

    /// Get the current brightness, possibly using a cached value if recent
    /// enough
    pub fn brightness_maybe_cached(&mut self) -> anyhow::Result<u32> {
        // If we have hardware monitoring, we can trust the most recent brightness
        // value.
        if self.hw_monitor_path.is_some() && self.most_recent_brightness.is_some() {
            return Ok(self.most_recent_brightness.unwrap().1);
        }
        // Otherwise, cache for 1 second.
        if let Some((time, brightness)) = self.most_recent_brightness {
            if time.elapsed().as_millis() < 1000 {
                return Ok(brightness);
            }
        }
        self.brightness()
    }

    /// Get the max brightness supported
    #[allow(unused)]
    pub fn max_brightness(&self) -> anyhow::Result<u32> {
        let mut p = self.path.clone();
        p.push(MAX_BRIGHTNESS);
        read_int_path(p.as_path())
    }

    /// Set the current brightness
    pub fn set_brightness(
        &mut self,
        brightness: u32,
        state: &mut State,
        adaptive_brightness: bool,
    ) -> anyhow::Result<()> {
        // If hardware monitoring is not supported, try reading the previous value.
        if adaptive_brightness && self.hw_monitor_path.is_none() && brightness == 0 {
            let old_brightness = read_int(&mut self.brightness_file)?;
            if old_brightness > 0 {
                state.on_brightness = old_brightness;
            }
        }
        self.brightness_file.rewind()?;
        write!(self.brightness_file, "{brightness}")?;
        self.most_recent_brightness = Some((Instant::now(), brightness));
        Ok(())
    }

    /// Get the path to monitor for HW changes. Not all LEDs support this.
    pub fn hw_monitor_path(&self) -> Option<&Path> {
        self.hw_monitor_path.as_deref()
    }

    /// Get the path to the brightness file, this is used to monitor for
    /// other user space programs changing the brightness level.
    pub fn sw_monitor_path(&self) -> PathBuf {
        let mut p = self.path.clone();
        p.push(BRIGHTNESS);
        p
    }
}
