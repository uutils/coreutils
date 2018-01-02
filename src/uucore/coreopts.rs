extern crate getopts;

use super::{ProgramInfo, UStatus};
use std::io::{self, Read, Write};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Getopts(#[cause] getopts::Fail),
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error)
}

impl UStatus for Error { }

generate_from_impl!(Error, Getopts, getopts::Fail);
generate_from_impl!(Error, Io, io::Error);

pub struct HelpText<'a> {
    pub name : &'a str,
    pub version : &'a str,
    pub syntax : &'a str,
    pub summary : &'a str,
    pub long_help : &'a str,
    pub display_usage : bool
}

pub struct CoreOptions<'a> {
    options : getopts::Options,
    help_text : HelpText<'a>
}

impl<'a> CoreOptions<'a> {
    pub fn new(help_text: HelpText<'a>) -> Self {
        let mut ret = CoreOptions {
            options : getopts::Options::new(),
            help_text : help_text
        };
        ret.options
            .optflag("", "help", "print usage information")
            .optflag("", "version", "print name and version number");
        ret
    }
    pub fn optflagopt(&mut self, short_name: &str, long_name: &str, desc: &str, hint: &str) -> &mut CoreOptions<'a> {
        self.options.optflagopt(short_name, long_name, desc, hint);
        self
    }
    pub fn optflag(&mut self, short_name: &str, long_name: &str, desc: &str) -> &mut CoreOptions<'a> {
        self.options.optflag(short_name, long_name, desc);
        self
    }
    pub fn optflagmulti(&mut self, short_name: &str, long_name: &str, desc: &str) -> &mut CoreOptions<'a> {
        self.options.optflagmulti(short_name, long_name, desc);
        self
    }
    pub fn optopt(&mut self, short_name: &str, long_name: &str, desc: &str, hint: &str) -> &mut CoreOptions<'a> {
        self.options.optopt(short_name, long_name, desc, hint);
        self
    }
    pub fn optmulti(&mut self, short_name: &str, long_name: &str, desc: &str, hint: &str) -> &mut CoreOptions<'a> {
        self.options.optmulti(short_name, long_name, desc, hint);
        self
    }
    pub fn usage(&self, summary : &str) -> String {
        self.options.usage(summary)
    }
    pub fn parse<'b, I, O, E>(&mut self, args : Vec<String>, pio: &mut ProgramInfo<'b, I, O, E>) -> Result<Option<getopts::Matches>, Error>//UError<CoreOptionsErrorKind>>//CoreOptionsError>
        where I: Read, O: Write, E: Write
    {
        let matches = self.options.parse(&args[1..])?;
        if matches.opt_present("help") {
            let usage_str = if self.help_text.display_usage {
                    format!("\n {}\n\n Reference\n",
                        self.options.usage(self.help_text.summary)
                    ).replace("Options:", " Options:")
                } else { String::new() };
            writeln!(pio, "
 {0} {1}

 {0} {2}
{3}{4}
", self.help_text.name, self.help_text.version, self.help_text.syntax, usage_str, self.help_text.long_help)?;
            Ok(None)
        } else if matches.opt_present("version") {
            writeln!(pio, "{} {}", self.help_text.name, self.help_text.version)?;
            Ok(None)
        } else {
            Ok(Some(matches))
        }
    }
}

#[macro_export]
macro_rules! new_coreopts {
    ($syntax: expr, $summary: expr, $long_help: expr) => (
        uucore::coreopts::CoreOptions::new(uucore::coreopts::HelpText {
            name: executable!(),
            version: env!("CARGO_PKG_VERSION"),
            syntax: $syntax,
            summary: $summary,
            long_help: $long_help,
            display_usage: true
        })
    );
    ($syntax: expr, $summary: expr, $long_help: expr, $display_usage: expr) => (
        uucore::coreopts::CoreOptions::new(uucore::coreopts::HelpText {
            name: executable!(),
            version: env!("CARGO_PKG_VERSION"),
            syntax: $syntax,
            summary: $summary,
            long_help: $long_help,
            display_usage: $display_usage
        })
    );
}
