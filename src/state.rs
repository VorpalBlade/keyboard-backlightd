//! Implements current state

use std::time::Instant;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct KeepCurrent {
    pub until: Instant,
}

#[derive(Debug)]
pub(crate) struct State {
    pub last_input: Instant,
    /// Describes the current keep-on/keep-off settings.
    pub keep: Option<KeepCurrent>,
    /// Brightness that we want.
    pub on_brightness: u32,
}

impl State {
    pub(crate) fn new() -> Self {
        Self {
            last_input: Instant::now(),
            keep: None,
            on_brightness: 0,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
