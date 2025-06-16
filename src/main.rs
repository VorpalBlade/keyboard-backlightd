//! Keyboard backlight daemon. Dims backlight after timeout on Thinkpads
//!
//! There is no public code API for you to use! However, the command line
//! interface should be stable.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;

use handlers::EvDevListener;
use handlers::HwBrightnessChangeListener;
use handlers::SwBrightnessChangeListener;
use monitor::monitor;
use state::State;

use crate::led::Led;
use crate::utils::wait_for_file;

mod flags;
mod handlers;
mod led;
mod monitor;
mod policy;
mod state;
mod utils;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = flags::Cli::parse();
    setup_daemon(&cli)?;
    Ok(())
}

/// Set up to start daemon
fn setup_daemon(config: &flags::Cli) -> anyhow::Result<()> {
    let mut evdev_listeners = vec![];
    let mut swbc_listener = None;
    let mut hwbc_listener = None;
    let mut state = State::new();

    state.on_brightness = config.brightness.unwrap_or(1);

    let devices_to_monitor = utils::normalize_devices(config.monitor_input.clone(), utils::get_default_devices()?)?;
    for e in devices_to_monitor {
        evdev_listeners.push(Some(EvDevListener::new(&e)?));
    }

    if let Some(timeout) = config.wait {
        wait_for_file(config.led_base_dir.as_path(), Duration::from_millis(timeout.into()))?;
    }
    let led = Rc::new(RefCell::new(
        Led::new(config.led_base_dir.clone()).context("Failed to create LED")?,
    ));

    if !config.no_adaptive_brightness {
        swbc_listener = Some(SwBrightnessChangeListener { led: led.clone() });
        if led.borrow().hw_monitor_path().is_some() {
            hwbc_listener = Some(HwBrightnessChangeListener { led: led.clone() });
        }
    }

    monitor(evdev_listeners, swbc_listener, hwbc_listener, state, led, config)?;

    unreachable!();
}
