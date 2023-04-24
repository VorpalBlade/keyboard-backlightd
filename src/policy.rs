//! This module computes the output action to perform.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use log::debug;
use smallvec::{smallvec, SmallVec};

use crate::{flags::KeyboardBacklightd, led::Led, state::State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PolicyAction {
    SetLed(u8),
    Sleep(Option<Duration>),
}

pub(crate) fn run_policy(
    state: &mut State,
    config: &KeyboardBacklightd,
    led: &Rc<RefCell<Led>>,
) -> anyhow::Result<Option<Duration>> {
    state.cur_brightness = led.borrow().brightness()?;

    let actions = policy(state, config);

    debug!("State: {state:?}");
    debug!("Actions: {actions:?}");

    for action in actions {
        match action {
            PolicyAction::SetLed(brightness) => {
                led.borrow_mut().set_brightness(brightness)?;
            }
            PolicyAction::Sleep(dur) => return Ok(dur),
        }
    }
    unreachable!("There should always be a sleep action!");
}

fn policy(state: &mut State, settings: &KeyboardBacklightd) -> SmallVec<[PolicyAction; 2]> {
    let now = Instant::now();
    let led_timeout = Duration::from_millis(settings.timeout.into());

    if let Some(keep) = &state.keep {
        if now > keep.until {
            state.keep = None;
        } else {
            return smallvec![
                PolicyAction::SetLed(state.requested_brightness),
                PolicyAction::Sleep(Some(keep.until - now))
            ];
        }
    }

    if (now - state.last_input) > led_timeout {
        // Turn off LED, sleep until input
        smallvec![PolicyAction::SetLed(0), PolicyAction::Sleep(None)]
    } else {
        smallvec![
            PolicyAction::SetLed(state.requested_brightness),
            PolicyAction::Sleep(Some(led_timeout))
        ]
    }
}
