//! MIO bindings for Unix Domain Sockets

#![cfg(unix)]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/mio-uds/0.6")]

extern crate iovec;
extern crate libc;
extern crate mio;

use std::io;

mod datagram;
mod listener;
mod socket;
mod stream;

pub use datagram::UnixDatagram;
pub use listener::UnixListener;
pub use stream::UnixStream;

fn cvt(i: libc::c_int) -> io::Result<libc::c_int> {
    if i == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(i)
    }
}
