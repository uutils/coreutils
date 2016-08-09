#![crate_name = "uu_arch"]

/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Smigle00 <smigle00@gmail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

extern crate libc;

#[macro_use]
extern crate uucore;

use std::ffi::CStr;
use std::mem::uninitialized;
use uucore::c_types::utsname;

struct Arch {
    arch_name: String
}

extern {
    fn uname(uts: *mut utsname);
}

unsafe fn string_from_c_str(ptr: *const i8) -> String {
    String::from_utf8_lossy(CStr::from_ptr(ptr as *const std::os::raw::c_char).to_bytes()).to_string()
}

unsafe fn get_machine_arch() -> Arch {
    let mut uts: utsname = uninitialized();
    uname(&mut uts);
    Arch {
        arch_name: string_from_c_str(uts.machine.as_ptr()  as *const i8)
    }
}

static NAME: &'static str = "arch";
static VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn uumain(args: Vec<String>) -> i32 {
    let mut opts = uucore::coreopts::CoreOptions::new();
    let usage = opts.usage("Determine architecture name for current machine.");
    opts.help(format!("
{0} {1}

{0}

{2}
", NAME, VERSION, usage)).parse(args);

    let machine_arch = unsafe { get_machine_arch() };
    let mut output = String::new();
    output.push_str(machine_arch.arch_name.as_ref());
    println!("{}", output.trim());

    0
}
