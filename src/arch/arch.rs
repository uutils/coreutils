#![crate_name = "uu_arch"]

// This file is part of the uutils coreutils package.
//
// (c) Smigle00 <smigle00@gmail.com>
// (c) Jian Zeng <anonymousknight96 AT gmail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
//

#[macro_use]
extern crate uucore;
extern crate failure;
#[macro_use]
extern crate failure_derive;

use uucore::utsname::Uname;
use uucore::{ProgramInfo, UStatus, Util};
use std::io::{self, Read, Write};

static SYNTAX: &'static str = "";
static SUMMARY: &'static str = "Determine architecture name for current machine.";
static LONG_HELP: &'static str = "";

pub const UTILITY: Arch = Arch;

pub struct Arch;

impl<'a, I: Read, O: Write, E: Write> Util<'a, I, O, E, ArchError> for Arch {
    fn uumain(args: Vec<String>, pio: &mut ProgramInfo<I, O, E>) -> Result<i32, ArchError> {
        if new_coreopts!(SYNTAX, SUMMARY, LONG_HELP).parse(args, pio)?.is_some() {
            let uts = Uname::new();
            writeln!(pio, "{}", uts.machine().trim())?;
        }
        Ok(0)
    }
}

#[derive(Debug, Fail)]
pub enum ArchError {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    #[fail(display = "{}", _0)]
    CoreOpts(#[cause] uucore::coreopts::Error)
}

impl UStatus for ArchError {
    fn code(&self) -> i32 {
        use ArchError::*;
        use uucore::coreopts::Error;

        match *self {
            Io(_) | CoreOpts(Error::Io(_)) => 1,
            CoreOpts(Error::Getopts(_)) => 2,
        }
    }
}

generate_from_impl!(ArchError, Io, io::Error);
generate_from_impl!(ArchError, CoreOpts, uucore::coreopts::Error);

//generate_error_type!(ArchError, uucore::coreopts::CoreOptionsError, _);
