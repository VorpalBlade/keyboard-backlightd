//! Main inotify/epoll loop

use std::cell::RefCell;
use std::os::fd::AsFd;
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
use udev::EventType;
use udev::MonitorBuilder;

use crate::flags::Cli;
use crate::led::Led;
use crate::policy::run_policy;
use crate::state::State;
use crate::utils::get_devnode_if_monitored;
use crate::EvDevListener;
use crate::HwBrightnessChangeListener;
use crate::SwBrightnessChangeListener;

/// Marker value in epoll for the inotify watch.
const INOTIFY_DATA: u64 = u64::MAX;
/// Marker value in epoll for the udev watch.
const UDEV_DATA:u64 = u64::MAX - 1;

/// Main loop that monitors all the different data sources.
pub(crate) fn monitor(
    // FIXME: A better representation. Currently,
    // 1. The Vec grows unbounded
    // 2. Can overflow (if there are 4B+ devices to watch)
    // 3. Each removed listener still occupies some space.
    mut evdev_listeners: Vec<Option<EvDevListener>>,
    mut sw_bcl: Option<SwBrightnessChangeListener>,
    mut hw_bcl: Option<HwBrightnessChangeListener>,
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

    let mut sw_bcl_wd = None;
    let mut hw_bcl_wd = None;

    // Add brightness change listeners to inotify
    if let Some(ref listener) = sw_bcl {
        sw_bcl_wd = Some(inotify.add_watch(&listener.led.borrow().sw_monitor_path(), AddWatchFlags::IN_MODIFY)?);
    }
    if let Some(ref listener) = hw_bcl {
        hw_bcl_wd = Some(inotify.add_watch(listener.led.borrow().hw_monitor_path().unwrap(), AddWatchFlags::IN_MODIFY)?);
    }

    // Add evdev listeners to epoll
    for (idx, listener) in evdev_listeners.iter().enumerate() {
        // TRICKY BIT: Data = 0 is used to indicate nothing happened.
        // We thus offset the array index into listeners by one.
        epoll.add(
            listener.as_ref().unwrap().dev.file().as_fd(),
            EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, (idx + 1) as u64),
        )?;
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
                        if sw_bcl_wd.is_some_and(|wd| wd == ievent.wd) {
                            sw_bcl.as_mut().unwrap().process(&mut state, &duration)?;
                        } else if hw_bcl_wd.is_some_and(|wd| wd == ievent.wd) {
                            hw_bcl.as_mut().unwrap().process(&mut state, &duration)?;
                        }
                    }
                },
                UDEV_DATA => {
                    for udev_event in udev_socket.iter() {
                        // We only monitor 'add' and 'remove' events
                        if ![EventType::Add, EventType::Remove].contains(&udev_event.event_type()) {
                            continue;
                        }

                        let devnode = get_devnode_if_monitored(&udev_event, &config.monitor_input);
                        if devnode.is_none() {
                            continue;
                        }
                        let devnode = devnode.unwrap();

                        if udev_event.event_type() == EventType::Add {
                            let idx = evdev_listeners.len();
                            let new_listener = EvDevListener::new(devnode)?;
                            epoll.add(
                                new_listener.dev.file().as_fd(),
                                EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, (idx + 1) as u64),
                            )?;
                            evdev_listeners.push(Some(new_listener));
                        } else if udev_event.event_type() == EventType::Remove {
                            let (idx, listener) = evdev_listeners.iter()
                                .enumerate()
                                .filter(|(_, l)| l.is_some())
                                .map(|(idx, l)| (idx, l.as_ref().unwrap()))
                                .find(|(_, l)| l.devnode == devnode).unwrap();
                            epoll.delete(listener.dev.file().as_fd())?;
                            evdev_listeners[idx] = None;
                        }
                    }
                },
                0 => (),
                idx => {
                    let l = evdev_listeners.get_mut((idx - 1) as usize).unwrap().as_mut().unwrap();
                    l.process(&mut state, &duration)?;
                }
            }
        }
        timeout = run_policy(&mut state, config, &led)?;
    }
}
