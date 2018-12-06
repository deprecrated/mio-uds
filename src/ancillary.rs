use std::os::unix::io::{RawFd, AsRawFd, IntoRawFd};
use libc::{gid_t, pid_t, uid_t};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
/// Ancillary messages expected to be received by recvmsg(2)
pub struct AncillaryExpect {
    /// Expect credentials to be received by this socket
    pub creds: bool,
    /// Maximum number of open file descriptors expected to be
    /// received on this socket
    pub fds: usize,
}

impl Default for AncillaryExpect {
    fn default() -> Self {
        AncillaryExpect {
            creds: false,
            fds: 0,
        }
    }
}
    
#[derive(Eq, PartialEq, Debug)]
/// Ancillary messages sent via sendmsg(2) or received vi recvmsg(2)
pub struct Ancillary {
    /// File descriptors transfered alongside this socket
    pub fds_in: Vec<RawFd>,
    /// Credentials transfered alongside this socket
    pub cred: Option<UCred>,
}

impl Ancillary {
    /// Returns an empty struct
    pub fn empty() -> Self {
        Ancillary {
            fds_in: Vec::new(),
            cred: None,
        }
    }

    #[inline]
    /// Adds open file descriptor to data to be sent
    pub fn add_fds<F: IntoRawFd>(&mut self, fds: Vec<F>) {
        self.fds_in.reserve(fds.len());
        for f in fds {
            self.fds_in.push(f.into_raw_fd());
            // TODO: we take ownership here, we need to close it at some point
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
/// Credentials transfered alongside a socket
pub struct UCred {
    /// PID (process ID) of the process
    pub pid: pid_t,
    /// UID (user ID) of the process
    pub uid: uid_t,
    /// GID (group ID) of the process
    pub gid: gid_t,
}

