use std::cmp;
use std::io::prelude::*;
use std::io;
use std::mem;
use std::os::unix::net;
use std::os::unix::prelude::*;
use std::ptr;
use std::path::Path;
use std::net::Shutdown;

use iovec::IoVec;
use iovec::unix as iovec;
use libc;
use mio::event::Evented;
use mio::unix::EventedFd;
use mio::{Poll, Token, Ready, PollOpt};

use cvt;
use socket::{sockaddr_un, Socket};
use ancillary::{AncillaryExpect, Ancillary, UCred};
use cmsg::{Cmsg, CmsgData};

/// A Unix stream socket.
///
/// This type represents a `SOCK_STREAM` connection of the `AF_UNIX` family,
/// otherwise known as Unix domain sockets or Unix sockets. This stream is
/// readable/writable and acts similarly to a TCP stream where reads/writes are
/// all in order with respect to the other connected end.
///
/// Streams can either be connected to paths locally or another ephemeral socket
/// created by the `pair` function.
///
/// A `UnixStream` implements the `Read`, `Write`, `Evented`, `AsRawFd`,
/// `IntoRawFd`, and `FromRawFd` traits for interoperating with other I/O code.
///
/// Note that all values of this type are typically in nonblocking mode, so the
/// `read` and `write` methods may return an error with the kind of
/// `WouldBlock`, indicating that it's not ready to read/write just yet.
#[derive(Debug)]
pub struct UnixStream {
    inner: net::UnixStream,
}

#[repr(C)]
struct ScmCredentials(libc::ucred);

impl ScmCredentials {
    fn has_data(&self) -> bool {
        // to my knowledge, pid=0 is not a valid value
        self.0.pid != 0
    }
}

impl Default for ScmCredentials {
    fn default() -> Self {
        ScmCredentials(libc::ucred {
            pid: 0,
            // Ensure we do not give root (uid 0) access to anyone
            uid: libc::uid_t::max_value(),
            gid: libc::uid_t::max_value(),
        })
    }
}

impl<'a> Into<UCred> for &'a ScmCredentials {
    #[inline]
    fn into(self) -> UCred {
        UCred{
            pid: self.0.pid,
            uid: self.0.uid,
            gid: self.0.gid,
        }
    }
}

impl From<UCred> for ScmCredentials {
    #[inline]
    fn from(cred: UCred) -> ScmCredentials {
        ScmCredentials(libc::ucred {
            pid: cred.pid,
            uid: cred.uid,
            gid: cred.gid,
        })
    }
}


trait CmsgT {
    fn cmsg(self) -> Cmsg;
}

impl CmsgT for AncillaryExpect {
    fn cmsg(self) -> Cmsg {
        let mut cmsg = Cmsg::default();

        cmsg.empty_fds(self.fds);
        // TODO: need to implement Ucred
        
        cmsg
    }
}

impl CmsgT for Ancillary {
    fn cmsg(mut self) -> Cmsg {
        let mut cmsg = Cmsg::default();
        cmsg.add_fds_raw(&self.fds_in);
        // TODO: implement ucred 

        cmsg
    }
}

trait CmsgDataT {
    fn data(&self) -> Ancillary;
}

impl CmsgDataT for Cmsg {
    fn data(&self) -> Ancillary {
        let mut iter = self.iter();
        let mut out = Ancillary::empty();

        while let Some(el) = iter.next() {
            println!("el= {:?}", el);
            match el {
                CmsgData::Fd(fds) => {
                    out.fds_in.extend_from_slice(fds);
                },
                _ => {} // TODO manage ucred
            }
        }

        out
    }
}

impl UnixStream {
    /// Connects to the socket named by `path`.
    ///
    /// The socket returned may not be readable and/or writable yet, as the
    /// connection may be in progress. The socket should be registered with an
    /// event loop to wait on both of these properties being available.
    pub fn connect<P: AsRef<Path>>(p: P) -> io::Result<UnixStream> {
        UnixStream::_connect(p.as_ref())
    }

    fn _connect(path: &Path) -> io::Result<UnixStream> {
        unsafe {
            let (addr, len) = try!(sockaddr_un(path));
            let socket = try!(Socket::new(libc::SOCK_STREAM));
            let addr = &addr as *const _ as *const _;
            match cvt(libc::connect(socket.fd(), addr, len)) {
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }

            Ok(UnixStream::from_raw_fd(socket.into_fd()))
        }
    }

    /// Consumes a standard library `UnixStream` and returns a wrapped
    /// `UnixStream` compatible with mio.
    ///
    /// The returned stream is moved into nonblocking mode and is otherwise
    /// ready to get associated with an event loop.
    pub fn from_stream(stream: net::UnixStream) -> io::Result<UnixStream> {
        try!(stream.set_nonblocking(true));
        Ok(UnixStream { inner: stream })
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two `UnixStream`s which are connected to each other.
    pub fn pair() -> io::Result<(UnixStream, UnixStream)> {
        Socket::pair(libc::SOCK_STREAM).map(|(a, b)| unsafe {
            (UnixStream::from_raw_fd(a.into_fd()),
             UnixStream::from_raw_fd(b.into_fd()))
        })
    }

    /// Creates a new independently owned handle to the underlying socket.
    ///
    /// The returned `UnixStream` is a reference to the same stream that this
    /// object references. Both handles will read and write the same stream of
    /// data, and options set on one stream will be propogated to the other
    /// stream.
    pub fn try_clone(&self) -> io::Result<UnixStream> {
        self.inner.try_clone().map(|s| {
            UnixStream { inner: s }
        })
    }

    /// Returns the socket address of the local half of this connection.
    pub fn local_addr(&self) -> io::Result<net::SocketAddr> {
        self.inner.local_addr()
    }

    /// Returns the socket address of the remote half of this connection.
    pub fn peer_addr(&self) -> io::Result<net::SocketAddr> {
        self.inner.peer_addr()
    }

    /// Returns the value of the `SO_ERROR` option.
    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the
    /// specified portions to immediately return with an appropriate value
    /// (see the documentation of `Shutdown`).
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }

    /// Read in a list of buffers all at once.
    ///
    /// This operation will attempt to read bytes from this socket and place
    /// them into the list of buffers provided. Note that each buffer is an
    /// `IoVec` which can be created from a byte slice.
    ///
    /// The buffers provided will be filled in sequentially. A buffer will be
    /// entirely filled up before the next is written to.
    ///
    /// The number of bytes read is returned, if successful, or an error is
    /// returned otherwise. If no bytes are available to be read yet then
    /// a "would block" error is returned. This operation does not block.
    pub fn read_bufs(&self, bufs: &mut [&mut IoVec], ancillary: AncillaryExpect) -> io::Result<(usize, Ancillary)> {
        unsafe {
            let slice = iovec::as_os_slice_mut(bufs);
            let len = cmp::min(<libc::c_int>::max_value() as usize, slice.len());

            let mut cmsg = ancillary.cmsg();

            let mut msg = libc::msghdr {
                msg_name: ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: slice.as_mut_ptr(),
                msg_iovlen: len,
                msg_control: cmsg.as_mut_slice().as_mut_ptr() as (*mut libc::c_void),
                msg_controllen: cmsg.len(),
                msg_flags: 0,
            };

            let flags = libc::MSG_DONTWAIT; // TODO: do I need this? (socket is probably already set non-blocking)
            let rc = libc::recvmsg(self.inner.as_raw_fd(),
                                   &mut msg,
                                   flags);
            if rc < 0 {
                Err(io::Error::last_os_error())
            } else {
                let ancillary = cmsg.data();
                Ok((rc as usize, ancillary))
            }
        }
    }

    /// Write a list of buffers all at once alongside ancillary data.
    ///
    /// This operation will attempt to write a list of byte buffers to this
    /// socket. Note that each buffer is an `IoVec` which can be created from a
    /// byte slice.
    ///
    /// The buffers provided will be written sequentially. A buffer will be
    /// entirely written before the next is written.
    ///
    /// The number of bytes written is returned, if successful, or an error is
    /// returned otherwise. If the socket is not currently writable then a
    /// "would block" error is returned. This operation does not block.
    pub fn write_bufs_ancillary(&self, bufs: &[&IoVec], ancillary: Ancillary) -> io::Result<usize> {
        unsafe {
            let slice = iovec::as_os_slice(bufs);
            let len = cmp::min(<libc::c_int>::max_value() as usize, slice.len());

            let mut cmsg = ancillary.cmsg();

            let msg = libc::msghdr {
                msg_name: ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: slice.as_ptr() as *mut libc::iovec,
                msg_iovlen: len,
                msg_control: cmsg.as_mut_slice().as_mut_ptr() as *mut libc::c_void,
                msg_controllen: cmsg.len(),
                msg_flags: 0,
            };

            let flags = libc::MSG_DONTWAIT;
            let rc = libc::sendmsg(self.inner.as_raw_fd(),
                                   &msg,
                                   flags);
            if rc < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(rc as usize)
            }
        }
    }

    /// Write a list of buffers all at once.
    ///
    /// This operation will attempt to write a list of byte buffers to this
    /// socket. Note that each buffer is an `IoVec` which can be created from a
    /// byte slice.
    ///
    /// The buffers provided will be written sequentially. A buffer will be
    /// entirely written before the next is written.
    ///
    /// The number of bytes written is returned, if successful, or an error is
    /// returned otherwise. If the socket is not currently writable then a
    /// "would block" error is returned. This operation does not block.
    pub fn write_bufs(&self, bufs: &[&IoVec]) -> io::Result<usize> {
        self.write_bufs_ancillary(bufs, Ancillary::empty())
    }

}

impl Evented for UnixStream {
    fn register(&self,
                poll: &Poll,
                token: Token,
                events: Ready,
                opts: PollOpt) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).register(poll, token, events, opts)
    }

    fn reregister(&self,
                  poll: &Poll,
                  token: Token,
                  events: Ready,
                  opts: PollOpt) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).reregister(poll, token, events, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).deregister(poll)
    }
}

impl Read for UnixStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.inner.read(bytes)
    }
}

impl<'a> Read for &'a UnixStream {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        (&self.inner).read(bytes)
    }
}

impl Write for UnixStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<'a> Write for &'a UnixStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        (&self.inner).write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.inner).flush()
    }
}

impl AsRawFd for UnixStream {
    fn as_raw_fd(&self) -> i32 {
        self.inner.as_raw_fd()
    }
}

impl IntoRawFd for UnixStream {
    fn into_raw_fd(self) -> i32 {
        self.inner.into_raw_fd()
    }
}

impl FromRawFd for UnixStream {
    unsafe fn from_raw_fd(fd: i32) -> UnixStream {
        UnixStream { inner: net::UnixStream::from_raw_fd(fd) }
    }
}
