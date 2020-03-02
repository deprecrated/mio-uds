//! Deprecated MIO bindings for Unix Domain Sockets

#![cfg(unix)]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/mio-uds/0.7")]

extern crate mio;

pub use mio::net::{UnixStream, UnixListener, UnixDatagram};
