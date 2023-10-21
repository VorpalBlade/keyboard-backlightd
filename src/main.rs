//! Keyboard backlight daemon. Dims backlight after timeout on Thinkpads
//!
//! There is no public code API for you to use! However, the command line
//! interface should be stable.

mod flags;
mod handlers;
mod led;
mod monitor;
mod policy;
mod state;
mod utils;

use std::{cell::RefCell, rc::Rc, time::Duration};

use anyhow::Context;
use handlers::{EvDevListener, Handler, HwChangeListener};
use monitor::monitor;
use state::State;

use crate::{led::Led, utils::wait_for_file};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    match flags::KeyboardBacklightd::from_env() {
        Ok(flags) => match flags.validate() {
            Ok(()) => setup_daemon(&flags)?,
            Err(err) => err.exit(),
        },
        Err(err) => err.exit(),
    }
    Ok(())
}

/// Set up to start daemon
fn setup_daemon(config: &flags::KeyboardBacklightd) -> anyhow::Result<()> {
    let mut listeners: Vec<Box<dyn Handler>> = vec![];
    let mut state = State::new();

    if let Some(brightness) = config.brightness {
        state.requested_brightness = brightness;
    } else {
        state.requested_brightness = 1;
    }

    for e in &config.monitor_input {
        if let Some(timeout) = config.wait {
            wait_for_file(e.as_path(), Duration::from_millis(timeout.into()))?;
        }
        listeners.push(Box::new(EvDevListener::new(e)?));
    }
    if let Some(timeout) = config.wait {
        wait_for_file(config.led.as_path(), Duration::from_millis(timeout.into()))?;
    }
    let led = Rc::new(RefCell::new(
        Led::new(config.led.clone()).context("Failed to create LED")?,
    ));
    if !config.no_adaptive_brightness {
        if let Some(hw_path) = led.borrow().monitor_path() {
            listeners.push(Box::new(HwChangeListener::new(hw_path.into(), led.clone())));
        }
    }

    monitor(listeners, state, led, config)?;

    unreachable!();
}
