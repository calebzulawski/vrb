// Inspired by https://github.com/lassik/shm_open_anon (ISC license, Copyright 2019 Lassi Kortela)

use nix::Result;
use std::os::unix::io::RawFd;

#[cfg(not(target_os = "freebsd"))]
fn shm_open_anonymous_posix() -> Result<RawFd> {
    use nix::{
        errno::Errno,
        fcntl::OFlag,
        sys::{
            mman::{shm_open, shm_unlink},
            stat::Mode,
        },
        time::{clock_gettime, ClockId},
        unistd::close,
        Error,
    };
    use std::{ffi::CStr, io::Write};
    let mut filename = *b"/shm-vrb-XXXX\0";
    for _ in 0..4 {
        // add some random characters to the filename
        let filename = {
            let time = clock_gettime(ClockId::CLOCK_REALTIME)?;
            write!(&mut filename[9..], "{:4}", time.tv_nsec() % 10000).unwrap();
            CStr::from_bytes_with_nul(filename.as_ref()).unwrap()
        };

        // open the file
        match shm_open(
            filename,
            OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_NOFOLLOW,
            Mode::from_bits(0o600).unwrap(),
        ) {
            Ok(fd) => {
                shm_unlink(filename).map_err(|err| {
                    let _ = close(fd);
                    err
                })?;
                return Ok(fd);
            }
            Err(Error::Sys(Errno::EEXIST)) => continue,
            error => {
                error?;
            }
        }
    }
    Err(Errno::EEXIST.into())
}

pub fn shm_open_anonymous() -> Result<RawFd> {
    #[cfg(target_os = "linux")]
    {
        use nix::{
            error::{Errno, Error},
            sys::memfd::{memfd_create, MemFdCreateFlag::MFD_CLOEXEC},
        };
        match memfd_create("shm-vrb", MFD_CLOEXEC) {
            Err(Error::Sys(Errno::ENOSYS)) => shm_open_anonymous_posix(),
            value => value,
        }
    }

    #[cfg(target_os = "freebsd")]
    {
        use libc::SHM_ANON;
        use nix::{fcntl::OFlag, sys::mman::shm_open};
        shm_open(SHM_ANON, OFlag::O_RDWR, 0)
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    shm_open_anonymous_posix()
}

#[cfg(test)]
mod test {
    #[test]
    fn shm_open_anonymous() {
        let fd = super::shm_open_anonymous().unwrap();
        assert!(fd != -1);
        nix::unistd::close(fd).unwrap();
    }
}
