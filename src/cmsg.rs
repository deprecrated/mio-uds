use std::io::Cursor;
use std::iter::Iterator;
use std::mem;
use std::os::unix::io::{AsRawFd, RawFd};
use std::slice;

use byteorder::{NativeEndian, ReadBytesExt};

use ancillary::UCred;


// TODO turns out it doesn't looks like there is a need for padding here
//#[cfg_attr(target_pointer_width="32", repr(align(4)))]
//#[cfg_attr(target_pointer_width="64", repr(align(8)))]
#[repr(C)]
struct ScmRight(RawFd);

impl ScmRight {
    fn has_data(&self) -> bool {
        self.0 != 0
    }
}

impl Default for ScmRight {
    fn default() -> Self {
        ScmRight(0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CmsgData<'a> {
    Fd(&'a [RawFd]),
    Cred(&'a UCred)
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Cmsg {
    buf: Vec<u8>
}

impl Cmsg {
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.buf.as_mut_slice()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl Default for Cmsg {
    fn default() -> Self {
        Cmsg {
            buf: Vec::new(),
        }
    }
}

impl Cmsg {
    pub fn iter<'a>(&'a self) -> CmsgIter<'a> {
        CmsgIter{
            cmsg: self,
            pos: 0,
        }
    }

    fn add<T>(&mut self, type_: libc::c_int, t_len: usize) -> &mut [T] {
        let data_len = t_len * mem::size_of::<T>();
        let header_len = mem::size_of::<libc::cmsghdr>();
        let total_len = header_len + data_len; 
        let hdr = libc::cmsghdr {
            cmsg_level: libc::SOL_SOCKET,
            cmsg_type: type_,
            cmsg_len: total_len,
        };

        // Copy the header in the buffer
        self.buf.reserve(total_len);
        let hdr_buf: &[u8] = unsafe {
            let hdr_ptr: *const u8 = mem::transmute(&hdr);
            slice::from_raw_parts(hdr_ptr, header_len)
        };
        self.buf.extend_from_slice(hdr_buf);

        // add space for data
        let after_hdr_len = self.buf.len();
        self.buf.resize(after_hdr_len + data_len, 0);

        // return a slice for client to fill in
        let complete_buffer = self.buf.as_mut_slice();
        let data_buffer = &mut complete_buffer[after_hdr_len..];
        let data_ptr: *mut u8 = data_buffer.as_mut_ptr();
        unsafe {
            slice::from_raw_parts_mut(data_ptr as *mut T, t_len)
        }
    }

    pub fn add_fds<F: AsRawFd>(&mut self, data: &[F]) {
        let mut data_buf: &mut [ScmRight] = self.add(libc::SCM_RIGHTS, data.len());

        for i in 0..data.len() {
            data_buf[i] = ScmRight(data[i].as_raw_fd());
        }
    }

    pub fn add_fds_raw(&mut self, data: &[RawFd]) {
        let mut data_buf: &mut [ScmRight] = self.add(libc::SCM_RIGHTS, data.len());

        for i in 0..data.len() {
            data_buf[i] = ScmRight(data[i]);
        }
    }

    pub fn empty_fds(&mut self, len: usize) {
        let _: &mut [ScmRight] = self.add(libc::SCM_RIGHTS, len);
    }
}

pub struct CmsgIter<'a> {
    cmsg: &'a Cmsg,
    pos: usize,
}

impl<'a> Iterator for CmsgIter<'a> {
    type Item = CmsgData<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let remaining = &self.cmsg.buf[self.pos..];
            if remaining.len() == 0 {
                return None;
            }

            let hdr_len = mem::size_of::<libc::cmsghdr>();

            if remaining.len() < hdr_len {
                panic!("not enough bytes for length");
            }

            let mut cur = Cursor::new(remaining);

            #[cfg(target_pointer_width="32")]
            let len = cur.read_u32::<NativeEndian>().unwrap() as usize;
            #[cfg(target_pointer_width="64")]
            let len = cur.read_u64::<NativeEndian>().unwrap() as usize;

            let level = cur.read_i32::<NativeEndian>().unwrap();
            let type_ = cur.read_i32::<NativeEndian>().unwrap();

            self.pos += len;
            if level != libc::SOL_SOCKET {
                continue;
            }
            
            match type_ {
                libc::SCM_RIGHTS => {
                    let remaining = cur.into_inner();
                    let data = &remaining[hdr_len..len];
                    let data_len = len - hdr_len;
                    let data = unsafe {
                        slice::from_raw_parts(data.as_ptr() as *const RawFd, data_len / mem::size_of::<RawFd>())
                    };

                    return Some(CmsgData::Fd(data));
                },
                _ => {
                    continue
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::os::unix::io::FromRawFd;
    use super::*;

    #[cfg_attr(target_pointer_width="64", cfg_attr(target_endian="little", test))]
    fn add_three_fd() {
        let mut cmsg = Cmsg::default();
        let fds = unsafe {vec![
            File::from_raw_fd(1),
            File::from_raw_fd(2),
            File::from_raw_fd(3)
        ]};
        cmsg.add_fds(&fds);
        
        assert_eq!(cmsg.buf, vec![
            28, 0, 0, 0, 0, 0, 0, 0, // len
            1, 0 ,0, 0, // level
            1, 0, 0, 0, // type

            // fds
            1, 0, 0, 0,
            2, 0, 0, 0,
            3, 0, 0, 0
        ]);
    }

    #[test]
    fn iter() {
        let mut cmsg = Cmsg::default();
        let fds = unsafe {vec![
            File::from_raw_fd(1),
            File::from_raw_fd(2),
        ]};
        cmsg.add_fds(&fds);

        let mut iterator = cmsg.iter();

        assert_eq!(Some(CmsgData::Fd(&[1, 2])), iterator.next());
        assert_eq!(None, iterator.next());
    }


}
