//! Handlers for activity on paths

use std::{error::Error, os::fd::BorrowedFd, path::Path, time::Duration};

use crate::state::State;

/// Describes what a handler wants to monitor.
pub(crate) enum ListenType<'a> {
    /// Monitor a file descriptor
    Fd(BorrowedFd<'a>),
    /// Monitor a path
    Path(&'a Path),
}

/// Handles some type of notification
pub(crate) trait Handler {
    /// List of FDs that needs to be monitored for this listener
    fn monitored(&self) -> ListenType;
    /// Called on change of the monitored thing
    fn process(&mut self, state: &mut State, dur: &Duration) -> Result<(), Box<dyn Error>>;
}

pub(crate) use ev_dev::EvDevListener;
pub(crate) use hw_change::HwChangeListener;

/// Code for handling /dev/input
mod ev_dev {
    use std::{
        error::Error,
        os::fd::AsFd,
        path::Path,
        time::{Duration, Instant},
    };

    use evdev_rs::{Device, ReadFlag};
    use log::warn;

    use crate::state::State;

    use super::{Handler, ListenType};

    /// Handler for /dev/input
    #[derive(Debug)]
    pub(crate) struct EvDevListener {
        dev: Device,
    }

    impl EvDevListener {
        pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
            Ok(Self {
                dev: Device::new_from_path(path)?,
            })
        }
    }

    impl Handler for EvDevListener {
        fn monitored(&self) -> ListenType {
            ListenType::Fd(self.dev.file().as_fd())
        }

        fn process(&mut self, state: &mut State, _dur: &Duration) -> Result<(), Box<dyn Error>> {
            let ev = self.dev.next_event(ReadFlag::NORMAL).map(|val| val.1);
            match ev {
                // This case is that an input event was reported, we don't care *which* one,
                // just that one happened and when.
                Ok(_) => {
                    // The time in the event is not monotonic, thus we need
                    // to get the time right now instead.
                    state.last_input = Instant::now();
                }
                Err(e) => warn!("Error reading {:?}: {}", self.dev.file(), e),
            }
            Ok(())
        }
    }
}

/// Code for handling /sys/class/leds/tpacpi::kbd_backlight/brightness_hw_changed (or similar files)
mod hw_change {
    use std::{
        cell::RefCell,
        error::Error,
        path::PathBuf,
        rc::Rc,
        time::{Duration, Instant},
    };

    use crate::{led::Led, state::State};

    use super::{Handler, ListenType};

    /// Handler for /sys/class/leds/tpacpi::kbd_backlight/brightness_hw_changed (or similar files
    #[derive(Debug)]
    pub(crate) struct HwChangeListener {
        path: PathBuf,
        led: Rc<RefCell<Led>>,
    }

    impl HwChangeListener {
        pub fn new(path: PathBuf, led: Rc<RefCell<Led>>) -> Self {
            Self { path, led }
        }
    }

    impl Handler for HwChangeListener {
        fn monitored(&self) -> ListenType {
            ListenType::Path(self.path.as_path())
        }

        fn process(&mut self, state: &mut State, _dur: &Duration) -> Result<(), Box<dyn Error>> {
            state.last_input = Instant::now();
            state.requested_brightness = self.led.borrow().brightness()?;
            Ok(())
        }
    }
}
