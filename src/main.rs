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

use std::{cell::RefCell, error::Error, rc::Rc};

use handlers::{EvDevListener, Handler, HwChangeListener};
use monitor::monitor;
use state::State;

use crate::led::Led;

fn main() -> Result<(), Box<dyn Error>> {
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
fn setup_daemon(config: &flags::KeyboardBacklightd) -> Result<(), Box<dyn Error>> {
    let mut listeners: Vec<Box<dyn Handler>> = vec![];
    let mut state = State::new();

    if let Some(brightness) = config.brightness {
        state.requested_brightness = brightness;
    } else {
        state.requested_brightness = 1;
    }

    for e in &config.monitor_input {
        listeners.push(Box::new(EvDevListener::new(e)?));
    }
    let led = Rc::new(RefCell::new(Led::new(config.led.clone())));
    if let Some(uefi_path) = led.borrow().monitor_path()? {
        listeners.push(Box::new(HwChangeListener::new(uefi_path, led.clone())));
    }

    monitor(listeners, state, led, config)?;

    unreachable!();
}
