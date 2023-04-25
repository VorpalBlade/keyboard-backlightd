//! Handlers for activity on paths

use std::{os::fd::BorrowedFd, path::Path, time::Duration};

use crate::state::State;

/// Describes what a handler wants to monitor.
#[derive(Debug)]
#[must_use]
pub(crate) enum ListenType<'a> {
    /// Monitor a file descriptor
    Fd(BorrowedFd<'a>),
    /// Monitor a path
    Path(&'a Path),
}

/// Describe how to modify the monitoring as a result of processing
#[derive(Debug)]
#[must_use]
pub(crate) enum ProcessAction<'a> {
    /// Continue monitoring as before
    NoChange,
    /// Change monitoring approach
    Reset { old: ListenType<'a> },
}

/// Handles some type of notification
pub(crate) trait Handler: std::fmt::Debug {
    /// List of FDs that needs to be monitored for this listener
    fn monitored(&self) -> ListenType;
    /// Called on change of the monitored thing
    fn process<'a>(
        &'a mut self,
        state: &mut State,
        dur: &Duration,
    ) -> anyhow::Result<ProcessAction<'a>>;

    /// Call to reset the monitoring. Needed to deal with messy real world,
    /// where device nodes can go away and have to be reopened.
    fn reopen(&mut self) -> anyhow::Result<()>;
}

pub(crate) use ev_dev::EvDevListener;
pub(crate) use hw_change::HwChangeListener;

/// Code for handling /dev/input
mod ev_dev {
    use std::{
        os::fd::AsFd,
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };

    use evdev_rs::{
        enums::{EventCode, EV_SYN},
        Device, ReadFlag,
    };
    use log::warn;

    use crate::state::State;

    use super::{Handler, ListenType, ProcessAction};

    /// Handler for /dev/input
    #[derive(Debug)]
    pub(crate) struct EvDevListener {
        path: PathBuf,
        dev: Device,
    }

    impl EvDevListener {
        pub fn new(path: &Path) -> anyhow::Result<Self> {
            Ok(Self {
                path: path.to_owned(),
                dev: Device::new_from_path(path)?,
            })
        }
    }

    impl Handler for EvDevListener {
        fn monitored(&self) -> ListenType {
            ListenType::Fd(self.dev.file().as_fd())
        }

        /// Reopen device, needed on some laptops after resume.
        fn reopen(&mut self) -> anyhow::Result<()> {
            warn!("Reopening {:?}", self.path);
            drop(&mut self.dev);
            self.dev = Device::new_from_path(self.path.as_path())?;
            Ok(())
        }

        fn process<'a>(
            &'a mut self,
            state: &mut State,
            _dur: &Duration,
        ) -> anyhow::Result<ProcessAction<'a>> {
            let ev = self.dev.next_event(ReadFlag::NORMAL).map(|val| val.1);
            match ev {
                // This case is that an input event was reported.
                Ok(key) => {
                    match key.event_code {
                        // If it was was a LED: Ignore it! (Otherwise pressing
                        // Caps Lock on an external USB keyboard would trigger
                        // us to turn on the backlight on the built in keyboard.
                        EventCode::EV_LED(_) => Ok(ProcessAction::NoChange),
                        // Similarly ignore SYN_REPORT, as these happen after
                        // each EV_LED (and in many other places too).
                        EventCode::EV_SYN(EV_SYN::SYN_REPORT) => Ok(ProcessAction::NoChange),
                        _ => {
                            // The time in the event is not monotonic, thus we need
                            // to get the time right now instead.
                            state.last_input = Instant::now();
                            Ok(ProcessAction::NoChange)
                        }
                    }
                }
                // This is a mess, since ENODEV isn't properly exposed via ErrorKind.
                Err(e) if e.raw_os_error() == Some(libc::ENODEV) => Ok(ProcessAction::Reset {
                    old: self.monitored(),
                }),
                Err(e) => {
                    warn!("Error reading {:?}: {}", self.dev.file(), e);
                    Err(e.into())
                }
            }
        }
    }
}

/// Code for handling /sys/class/leds/tpacpi::kbd_backlight/brightness_hw_changed (or similar files)
mod hw_change {
    use std::{
        cell::RefCell,
        path::PathBuf,
        rc::Rc,
        time::{Duration, Instant},
    };

    use crate::{led::Led, state::State};

    use super::{Handler, ListenType, ProcessAction};

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

        fn process<'a>(
            &'a mut self,
            state: &mut State,
            _dur: &Duration,
        ) -> anyhow::Result<ProcessAction<'a>> {
            state.last_input = Instant::now();
            state.requested_brightness = self.led.borrow_mut().brightness()?;
            Ok(ProcessAction::NoChange)
        }

        fn reopen(&mut self) -> anyhow::Result<()> {
            // No need to reopen anything in this case
            Ok(())
        }
    }
}
