//! Keyboard backlight daemon. Dims backlight after timeout on Thinkpads
//!
//! There is no public code API for you to use! However, the command line
//! interface should be stable.

use std::cell::RefCell;
use std::path::Path;
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
    let mut cli = flags::Cli::parse();
    setup_daemon(&mut cli)?;
    Ok(())
}

/// Set up to start daemon
fn setup_daemon(config: &mut flags::Cli) -> anyhow::Result<()> {
    let mut evdev_listeners = vec![];
    let mut swbc_listener = None;
    let mut hwbc_listener = None;
    let mut state = State::new();

    state.on_brightness = config.brightness.unwrap_or(1);

    let devices_to_monitor = utils::normalize_devices(config.monitor_input.clone(), utils::get_default_devices()?)?;
    for e in devices_to_monitor {
        evdev_listeners.push(Some(EvDevListener::new(&e)?));
    }

    if config.led_base_dir.is_none() {
        let leds_dir = Path::new("/sys/class/leds").to_path_buf();
        let mut kbd_led_dir = None;

        for entry in leds_dir.read_dir()? {
            let file_name = entry?.file_name().to_string_lossy().into_owned();
            if file_name.ends_with("kbd_backlight") {
                if kbd_led_dir.is_some() {
                    anyhow::bail!("Multiple kbd_backlights found. Please specify one explicitly.");
                } else {
                    kbd_led_dir = Some(file_name);
                }
            }
        }
        if kbd_led_dir.is_none() {
            anyhow::bail!("No kdb_backlight found. Please specify one explicitly.");
        }

        config.led_base_dir = Some(leds_dir.join(kbd_led_dir.unwrap()));
    }
    if let Some(timeout) = config.wait {
        wait_for_file(config.led_base_dir.as_ref().unwrap().as_path(), Duration::from_millis(timeout.into()))?;
    }
    let led = Rc::new(RefCell::new(
        Led::new(config.led_base_dir.as_ref().unwrap().clone()).context("Failed to create LED")?,
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
