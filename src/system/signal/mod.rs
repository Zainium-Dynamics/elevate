//! Utilities to handle signals.
mod handler;
mod info;
mod set;
mod state;
mod stream;

pub(crate) use handler::{SignalHandler, SignalHandlerBehavior};
pub(crate) use set::{SignalSet, take_first_pending};
pub(crate) use state::SignalsState;
pub(crate) use stream::{SignalStream, register_handlers};

use std::borrow::Cow;
use std::convert::Infallible;
use std::ffi::c_int;

use crate::system::signal::set::SignalAction;

pub(crate) type SignalNumber = c_int;

macro_rules! define_consts {
    ($($signal:ident,)*) => {
        pub(crate) mod consts {
            pub(crate) use libc::{$($signal,)*};
        }

        pub(crate) fn signal_name(signal: SignalNumber) -> Cow<'static, str> {
            match signal {
                $(consts::$signal => stringify!($signal).into(),)*
                _ => format!("unknown signal ({signal})").into(),
            }
        }
    };
}

define_consts! {
    SIGINT,
    SIGQUIT,
    SIGTSTP,
    SIGTERM,
    SIGHUP,
    SIGALRM,
    SIGPIPE,
    SIGUSR1,
    SIGUSR2,
    SIGCHLD,
    SIGCONT,
    SIGWINCH,
    SIGTTIN,
    SIGTTOU,
    SIGKILL,
    SIGSTOP,
}

pub(crate) fn exit_with_signal(signal: i32) -> Result<Infallible, crate::common::Error> {
    // Try to restore signal handler to the default to make the kill below actually kill the process
    // rather than run a signal handler.
    let _ = (|| SignalAction::new(SignalHandlerBehavior::Default)?.register(signal))();

    // The command already dumped core if it was going to; elevate itself must not.
    let no_core = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: setrlimit only reads the rlimit struct passed to it.
    unsafe { libc::setrlimit(libc::RLIMIT_CORE, &no_core) };

    crate::system::kill(crate::system::Process::process_id(), signal)?;
    unreachable!();
}
