//! xflags argument parsing
use std::path::PathBuf;

xflags::xflags! {
    /// Keyboard backlight daemon. Dims backlight after timeout on Thinkpads
    cmd keyboard-backlightd {
        /// Paths to evdev devices to monitor. Use /dev/input/by-id or /dev/input/by-path.
        repeated -i, --monitor-input path: PathBuf
        /// Path for LED to control.
        required -l, --led path: PathBuf
        /// Timeout in milliseconds before backlight is turned off.
        required -t, --timeout milliseconds: u32
        /// Brightness value to use by default.
        optional -b, --brightness value: u32
        /// Do not adapt to brightness changes performed externally by the user
        optional --no-adaptive-brightness
        /// Enable extra verbosity!
        optional -v, --verbose
        /// Timeout during startup for device nodes to appear.
        ///
        /// This can help with late loaded kernel modules.
        optional -w, --wait timeout: u32
    }
}

impl KeyboardBacklightd {
    pub fn validate(&self) -> xflags::Result<()> {
        if self.monitor_input.is_empty() {
            return Err(xflags::Error::new(
                "At least one monitored input (`-i`) is required",
            ));
        }
        Ok(())
    }
}
