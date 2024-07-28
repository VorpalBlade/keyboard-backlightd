//! Handlers for activity on paths

use std::os::fd::BorrowedFd;
use std::path::Path;
use std::time::Duration;

pub(crate) use ev_dev::EvDevListener;
pub(crate) use fs_change::HwChangeListener;
pub(crate) use fs_change::SwChangeListener;

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
    fn process(&mut self, state: &mut State, dur: &Duration) -> anyhow::Result<()>;
}

/// Code for handling /dev/input
mod ev_dev {
    use std::os::fd::AsFd;
    use std::path::Path;
    use std::time::Duration;
    use std::time::Instant;

    use anyhow::Context;
    use evdev_rs::enums::EventCode;
    use evdev_rs::enums::EV_SYN;
    use evdev_rs::Device;
    use evdev_rs::ReadFlag;
    use log::error;

    use crate::state::State;

    use super::Handler;
    use super::ListenType;

    /// Handler for /dev/input
    #[derive(Debug)]
    pub(crate) struct EvDevListener {
        dev: Device,
    }

    impl EvDevListener {
        pub fn new(path: &Path) -> anyhow::Result<Self> {
            Ok(Self {
                dev: Device::new_from_path(path)?,
            })
        }
    }

    impl Handler for EvDevListener {
        fn monitored(&self) -> ListenType {
            ListenType::Fd(self.dev.file().as_fd())
        }

        fn process(&mut self, state: &mut State, _dur: &Duration) -> anyhow::Result<()> {
            let ev = self.dev.next_event(ReadFlag::NORMAL).map(|val| val.1);
            match ev {
                // This case is that an input event was reported.
                Ok(key) => {
                    match key.event_code {
                        // If it was was a LED: Ignore it! (Otherwise pressing
                        // Caps Lock on an external USB keyboard would trigger
                        // us to turn on the backlight on the built in keyboard.
                        EventCode::EV_LED(_) => Ok(()),
                        // Similarly ignore SYN_REPORT, as these happen after
                        // each EV_LED (and in many other places too).
                        EventCode::EV_SYN(EV_SYN::SYN_REPORT) => Ok(()),
                        _ => {
                            // The time in the event is not monotonic, thus we need
                            // to get the time right now instead.
                            state.last_input = Instant::now();
                            Ok(())
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading {:?}: {}", self.dev.file(), e);
                    Err(e).with_context(|| format!("Error while reading {:?}", self.dev.file()))
                }
            }
        }
    }
}

/// Code for handling
/// /sys/class/leds/tpacpi::kbd_backlight/brightness_hw_changed (or similar
/// files)
mod fs_change {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::time::Duration;
    use std::time::Instant;

    use crate::led::Led;
    use crate::state::State;

    use super::Handler;
    use super::ListenType;

    /// Handler for /sys/class/leds/tpacpi::kbd_backlight/brightness_hw_changed
    /// (or similar files)
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

        fn process(&mut self, state: &mut State, _dur: &Duration) -> anyhow::Result<()> {
            state.last_input = Instant::now();
            state.requested_brightness = self.led.borrow_mut().brightness()?;
            Ok(())
        }
    }

    /// Handler for other userspace changing the brightness file
    #[derive(Debug)]
    pub(crate) struct SwChangeListener {
        path: PathBuf,
        led: Rc<RefCell<Led>>,
    }

    impl SwChangeListener {
        pub fn new(path: PathBuf, led: Rc<RefCell<Led>>) -> Self {
            Self { path, led }
        }
    }

    impl Handler for SwChangeListener {
        fn monitored(&self) -> ListenType {
            ListenType::Path(self.path.as_path())
        }

        fn process(&mut self, _state: &mut State, _dur: &Duration) -> anyhow::Result<()> {
            self.led.borrow_mut().brightness()?;
            Ok(())
        }
    }
}
