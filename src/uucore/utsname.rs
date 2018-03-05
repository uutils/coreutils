// This file is part of the uutils coreutils package.
//
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

pub use self::platform::*;

#[cfg(unix)]
mod platform {
    use ::libc::{uname, utsname};
    use ::std::mem;
    use ::std::ffi::CStr;
    use ::std::borrow::Cow;
    use ::std::io;

    macro_rules! cstr2cow {
        ($v:expr) => (
            unsafe { CStr::from_ptr($v.as_ref().as_ptr()).to_string_lossy() }
        )
    }

    pub struct Uname {
        inner: utsname,
    }

    impl Uname {
        pub fn new() -> io::Result<Self> {
            unsafe {
                let mut uts: utsname = mem::uninitialized();
                if uname(&mut uts) == 0 {
                    Ok(Uname { inner: uts })
                } else {
                    Err(io::Error::last_os_error())
                }
            }
        }

        pub fn sysname(&self) -> Cow<str> {
            cstr2cow!(self.inner.sysname)
        }

        pub fn nodename(&self) -> Cow<str> {
            cstr2cow!(self.inner.nodename)
        }

        pub fn release(&self) -> Cow<str> {
            cstr2cow!(self.inner.release)
        }

        pub fn version(&self) -> Cow<str> {
            cstr2cow!(self.inner.version)
        }

        pub fn machine(&self) -> Cow<str> {
            cstr2cow!(self.inner.machine)
        }
    }
}

#[cfg(windows)]
mod platform {
    use ::winapi::um::sysinfoapi::{SYSTEM_INFO, GetSystemInfo};
    use ::winapi::um::winnt::*;
    use ::std::mem;
    use ::std::borrow::Cow;
    use ::std::io;

    pub struct Uname {
        inner: SYSTEM_INFO
    }

    impl Uname {
        pub fn new() -> io::Result<Uname> {
            unsafe {
                let mut info = mem::uninitialized();
                GetSystemInfo(&mut info);
                Ok(Uname { inner: info })
            }
        }

        // FIXME: need to implement more architectures (e.g. ARM)
        pub fn machine(&self) -> Cow<str> {
            let arch = unsafe {
                match self.inner.u.s().wProcessorArchitecture {
                    PROCESSOR_ARCHITECTURE_AMD64 => "x86_64",
                    PROCESSOR_ARCHITECTURE_INTEL => "x86",
                    _ => unimplemented!()
                }
            };
            Cow::from(arch)
        }
    }
}