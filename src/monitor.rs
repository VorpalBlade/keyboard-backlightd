//! Main inotify/epoll loop

use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::AsFd;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;

use nix::errno::Errno;
use nix::sys::epoll::Epoll;
use nix::sys::epoll::EpollCreateFlags;
use nix::sys::epoll::EpollEvent;
use nix::sys::epoll::EpollFlags;
use nix::sys::epoll::EpollTimeout;
use nix::sys::inotify::AddWatchFlags;
use nix::sys::inotify::InitFlags;
use nix::sys::inotify::Inotify;
use udev::Event;
use udev::MonitorBuilder;

use crate::flags::Cli;
use crate::handlers::Handler;
use crate::led::Led;
use crate::policy::run_policy;
use crate::state::State;

/// Marker value in epoll for the inotify watch.
const INOTIFY_DATA: u64 = u64::MAX;
/// Marker value in epoll for the udev watch.
const UDEV_DATA:u64 = u64::MAX - 1;

/// Main loop that monitors all the different data sources.
pub(crate) fn monitor(
    mut listeners: Vec<Box<dyn Handler>>,
    mut state: State,
    led: Rc<RefCell<Led>>,
    config: &Cli,
) -> anyhow::Result<()> {
    let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC)?;

    let inotify = Inotify::init(InitFlags::IN_CLOEXEC | InitFlags::IN_NONBLOCK)?;
    epoll.add(
        inotify.as_fd(),
        EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, INOTIFY_DATA),
    )?;

    let udev_socket = MonitorBuilder::new()?
        .match_subsystem("input")?
        .listen()?;
    epoll.add(
        udev_socket.as_fd(),
        EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, UDEV_DATA),
    )?;

    let mut inotify_map = HashMap::new();

    // Add all the listeners
    for (idx, listener) in listeners.iter().enumerate() {
        match listener.monitored() {
            crate::handlers::ListenType::Fd(ref fd) => {
                // TRICKY BIT: Data = 0 is used to indicate nothing happened.
                // We thus offset the array index into listeners by one.
                epoll.add(
                    fd,
                    EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, (idx + 1) as u64),
                )?;
            }
            crate::handlers::ListenType::Path(p) => {
                inotify_map.insert(inotify.add_watch(p, AddWatchFlags::IN_MODIFY)?, idx);
            }
        }
    }

    let mut timeout = Some(Duration::ZERO);
    'main_loop: loop {
        let mut events = [EpollEvent::empty(); 32];
        let now = Instant::now();
        // TODO: Fixed timeout is wrong.
        match epoll.wait(
            &mut events,
            timeout.and_then(|e| TryInto::<EpollTimeout>::try_into(e).ok()),
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
                INOTIFY_DATA => {
                    for ievent in inotify.read_events()? {
                        let idx = inotify_map.get(&ievent.wd).unwrap();
                        let l = listeners.get_mut(*idx).unwrap();
                        l.process(&mut state, &duration)?;
                    }
                },
                UDEV_DATA => {
                    todo!();
                },
                0 => (),
                idx => {
                    let l = listeners.get_mut((idx - 1) as usize).unwrap();
                    l.process(&mut state, &duration)?;
                }
            }
        }
        timeout = run_policy(&mut state, config, &led)?;
    }
}
