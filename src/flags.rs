//! xflags argument parsing
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
/// Keyboard backlight daemon. Dims backlight after timeout on Thinkpads
pub struct Cli {
    /// Paths to evdev devices to monitor. Use /dev/input/by-id or
    /// /dev/input/by-path.
    #[clap(short = 'i', long)]
    pub monitor_input: Vec<PathBuf>,
    /// Path for LED to control.
    #[clap(short, long = "led")]
    pub led_base_dir: Option<PathBuf>,
    /// Timeout in milliseconds before backlight is turned off.
    #[clap(short, long)]
    pub timeout: u32,
    /// Brightness value to use by default.
    #[clap(short, long)]
    pub brightness: Option<u32>,
    /// Do not adapt to brightness changes performed externally by the user
    #[clap(long)]
    pub no_adaptive_brightness: bool,
    /// Enable extra verbosity!
    #[clap(short, long)]
    pub verbose: bool,
    /// Timeout during startup for device nodes to appear.
    ///
    /// This can help with late loaded kernel modules.
    #[clap(short, long)]
    pub wait: Option<u32>,
}
