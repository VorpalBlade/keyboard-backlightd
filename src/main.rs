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
use handlers::Handler;
use handlers::HwChangeListener;
use handlers::SwChangeListener;
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
    cli.validate()?;
    setup_daemon(&cli)?;
    Ok(())
}

/// Set up to start daemon
fn setup_daemon(config: &flags::Cli) -> anyhow::Result<()> {
    let mut listeners: Vec<Box<dyn Handler>> = vec![];
    let mut state = State::new();

    state.on_brightness = config.brightness.unwrap_or(1);

    for e in &config.monitor_input {
        listeners.push(Box::new(EvDevListener::new(e)?));
    }
    if let Some(timeout) = config.wait {
        wait_for_file(config.led_base_dir.as_path(), Duration::from_millis(timeout.into()))?;
    }
    let led = Rc::new(RefCell::new(
        Led::new(config.led_base_dir.clone()).context("Failed to create LED")?,
    ));
    if !config.no_adaptive_brightness {
        if let Some(hw_path) = led.borrow().hw_monitor_path() {
            listeners.push(Box::new(HwChangeListener::new(hw_path.into(), led.clone())));
            listeners.push(Box::new(SwChangeListener::new(
                led.borrow().sw_monitor_path(),
                led.clone(),
            )));
        }
    }

    monitor(listeners, state, led, config)?;

    unreachable!();
}
