//! MIO bindings for Unix Domain Sockets

#![cfg(unix)]
#![deny(missing_docs)]

extern crate libc;
extern crate mio;

use std::io;

mod datagram;
mod listener;
mod socket;
mod stream;

pub use stream::UnixStream;
pub use listener::UnixListener;
pub use datagram::UnixDatagram;

fn cvt(i: libc::c_int) -> io::Result<libc::c_int> {
    if i == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(i)
    }
}
