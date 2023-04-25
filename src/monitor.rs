//! Main inotify/epoll loop

use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsRawFd, BorrowedFd},
    rc::Rc,
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    sys::{
        epoll::{EpollCreateFlags, EpollFlags},
        inotify::{AddWatchFlags, InitFlags, Inotify, WatchDescriptor},
    },
};

use crate::{
    flags::KeyboardBacklightd,
    handlers::{Handler, ListenType, ProcessAction},
    led::Led,
    nix_polyfill::{Epoll, EpollEvent},
    policy::run_policy,
    state::State,
};

/// Marker value in epoll for the inotify watch.
const INOTIFY_HANDLE: u64 = u64::MAX;

#[derive(Debug)]
pub(crate) struct Monitor {
    /// LED object being controlled.
    led: Rc<RefCell<Led>>,
    inotify: Inotify,
    epoll: Epoll,
    /// Mapping of inotify watch descriptors to listener indices
    inotify_map: HashMap<WatchDescriptor, usize>,
}

impl Monitor {
    pub(crate) fn new(led: Rc<RefCell<Led>>) -> anyhow::Result<Self> {
        Ok(Self {
            led,
            inotify: Inotify::init(InitFlags::IN_CLOEXEC | InitFlags::IN_NONBLOCK)?,
            epoll: Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC)?,
            inotify_map: HashMap::new(),
        })
    }

    fn setup(&mut self, listeners: &[Box<dyn Handler>]) -> anyhow::Result<()> {
        // SAFETY: Epoll and inotify lives equally long. Also this cannot create a memory error anyway.
        //         This is safe.
        let ifd = unsafe { BorrowedFd::borrow_raw(self.inotify.as_raw_fd()) };

        self.epoll.add(
            ifd,
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, INOTIFY_HANDLE),
        )?;

        // Add all the listeners
        for (idx, listener) in listeners.iter().enumerate() {
            self.add_handler(listener.monitored(), idx)?;
        }
        Ok(())
    }

    /// Add a handler
    fn add_handler(&mut self, listen_type: ListenType, idx: usize) -> Result<(), anyhow::Error> {
        match listen_type {
            crate::handlers::ListenType::Fd(ref fd) => {
                // TRICKY BIT: Data = 0 is used to indicate nothing happend.
                // We thus offset the array index into listeners by one.
                self.epoll.add(
                    fd,
                    EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, (idx + 1) as u64),
                )?;
            }
            crate::handlers::ListenType::Path(p) => {
                self.inotify_map
                    .insert(self.inotify.add_watch(p, AddWatchFlags::IN_MODIFY)?, idx);
            }
        };
        Ok(())
    }

    /// Remove a handler
    fn remove_handler(
        &mut self,
        listen_type: ListenType,
        _idx: usize,
    ) -> Result<(), anyhow::Error> {
        match listen_type {
            ListenType::Fd(fd) => self.epoll.delete(fd)?,
            ListenType::Path(_) => {
                todo!("Removing inotify handler case not yet supported: {listen_type:?}")
            }
        }
        Ok(())
    }

    /// Main loop that monitors all the different data sources.
    pub(crate) fn monitor(
        &mut self,
        mut listeners: Vec<Box<dyn Handler>>,
        mut state: State,
        config: &KeyboardBacklightd,
    ) -> anyhow::Result<()> {
        self.setup(&listeners)?;

        let mut timeout = Some(Duration::ZERO);
        'main_loop: loop {
            let mut events = [EpollEvent::empty(); 32];
            let now = Instant::now();
            // TODO: Fixed timeout is wrong.
            match self.epoll.wait(
                &mut events,
                timeout
                    .map(|x: Duration| x.as_millis() as isize)
                    .unwrap_or(-1isize),
            ) {
                Ok(_) => (),
                Err(Errno::EINTR) => {
                    // Retry.
                    if let Some(t) = timeout {
                        timeout = Some(t.saturating_sub(now.elapsed()));
                    }
                    continue 'main_loop;
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Epoll error code: {err}"));
                }
            }
            let duration = now.elapsed();
            // Process events
            for ref event in events {
                match event.data() {
                    INOTIFY_HANDLE => {
                        for ievent in self.inotify.read_events()? {
                            let idx = *self.inotify_map.get(&ievent.wd).unwrap();
                            let l = listeners.get_mut(idx).unwrap();
                            match l.process(&mut state, &duration)? {
                                ProcessAction::NoChange => (),
                                ProcessAction::Reset { old } => {
                                    self.remove_handler(old, idx)?;
                                    l.reopen()?;
                                    let new = l.monitored();
                                    self.add_handler(new, idx)?;
                                }
                            }
                        }
                    }
                    0 => (),
                    idx => {
                        let l = listeners.get_mut((idx - 1) as usize).unwrap();
                        match l.process(&mut state, &duration)? {
                            ProcessAction::NoChange => (),
                            ProcessAction::Reset { old } => {
                                self.remove_handler(old, idx as usize)?;
                                l.reopen()?;
                                let new = l.monitored();
                                self.add_handler(new, idx as usize)?;
                            }
                        }
                    }
                }
            }
            timeout = run_policy(&mut state, config, &self.led)?;
        }
    }
}
