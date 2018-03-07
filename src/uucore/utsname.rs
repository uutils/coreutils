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

#[cfg(target_os = "redox")]
mod platform {
    use ::std::borrow::Cow;
    use ::std::io::{self, Read};
    use ::std::fs::File;

    pub struct Uname {
        kernel_name: String,
        nodename: String,
        kernel_release: String,
        kernel_version: String,
        machine: String
    }

    impl Uname {
        pub fn new() -> io::Result<Uname> {
            let mut inner = String::new();
            File::open("sys:uname")?.read_to_string(&mut inner)?;

            let mut lines = inner.lines();

            let kernel_name = lines.next().unwrap();
            let nodename = lines.next().unwrap();
            let kernel_release = lines.next().unwrap();
            let kernel_version = lines.next().unwrap();
            let machine = lines.next().unwrap();

            // FIXME: don't actually duplicate the data as doing so is wasteful
            Ok(Uname {
                kernel_name: kernel_name.to_owned(),
                nodename: nodename.to_owned(),
                kernel_release: kernel_release.to_owned(),
                kernel_version: kernel_version.to_owned(),
                machine: machine.to_owned()
            })
        }

        pub fn sysname(&self) -> Cow<str> {
            Cow::from(self.kernel_name.as_str())
        }

        pub fn nodename(&self) -> Cow<str> {
            Cow::from(self.nodename.as_str())
        }

        pub fn release(&self) -> Cow<str> {
            Cow::from(self.kernel_release.as_str())
        }

        pub fn version(&self) -> Cow<str> {
            Cow::from(self.kernel_version.as_str())
        }

        pub fn machine(&self) -> Cow<str> {
            Cow::from(self.machine.as_str())
        }
    }
}