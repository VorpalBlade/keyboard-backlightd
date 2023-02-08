//! Main inotify/epoll loop

use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    os::fd::{BorrowedFd, FromRawFd},
    rc::Rc,
    time::{Duration, Instant},
};

use nix::{
    errno::Errno,
    libc,
    sys::{
        epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
        inotify::{AddWatchFlags, InitFlags, Inotify},
    },
};

use crate::{
    flags::KeyboardBacklightd, handlers::Handler, led::Led, policy::run_policy, state::State,
};

// The API of nix sucks badly here. There is no way to get the FD out of inotify and into epoll
// We have to stupidly resort to unsafe code.
//
// This is based on Inotify::init
//
// SAFETY: Well, it actully isn't unsafe, since we aren't doing silly things like mmaping
//         this etc. It won't lead to memory safety.
//         HOWEVER: The IO safety is wrong, we are creating two copies of the same FD.
//         This can only be fixed in nix itself. But the way we use it is okay.
unsafe fn create_inotify<'a>(
    flags: InitFlags,
) -> Result<(Inotify, BorrowedFd<'a>), Box<dyn Error>> {
    let fd = Errno::result(unsafe { libc::inotify_init1(flags.bits()) })?;
    Ok(unsafe { (Inotify::from_raw_fd(fd), BorrowedFd::borrow_raw(fd)) })
}

const INOTIFY_HANDLE: u64 = u64::MAX;

pub(crate) fn monitor(
    mut listeners: Vec<Box<dyn Handler>>,
    mut state: State,
    led: Rc<RefCell<Led>>,
    config: &KeyboardBacklightd,
) -> Result<(), Box<dyn Error>> {
    // SAFETY: Epoll and inotify lives equally long. This is safe.
    let (inotify, ifd) = unsafe { create_inotify(InitFlags::IN_CLOEXEC | InitFlags::IN_NONBLOCK)? };
    let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC)?;

    epoll.add(
        ifd,
        EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, INOTIFY_HANDLE),
    )?;

    let mut inotify_map = HashMap::new();

    // Add all the listeners
    for (idx, listener) in listeners.iter().enumerate() {
        match listener.monitored() {
            crate::handlers::ListenType::Fd(ref fd) => {
                // TRICKY BIT: Data = 0 is used to indicate nothing happend.
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
            timeout.map(|x| x.as_millis() as isize).unwrap_or(-1isize),
        ) {
            Ok(_) => (),
            Err(Errno::EINTR) => {
                // Retry.
                if let Some(t) = timeout {
                    timeout = Some(t - now.elapsed());
                }
                continue 'main_loop;
            }
            Err(err) => {
                return Err(Box::new(err));
            }
        }
        let duration = now.elapsed();
        // Process events
        for ref event in events {
            match event.data() {
                INOTIFY_HANDLE => {
                    for ievent in inotify.read_events()? {
                        let idx = inotify_map.get(&ievent.wd).unwrap();
                        let l = listeners.get_mut(*idx).unwrap();
                        l.process(&mut state, &duration)?;
                    }
                }
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
