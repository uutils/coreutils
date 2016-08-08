extern crate getopts;
use std::io::Write;

pub struct CoreOptions {
    pub options : getopts::Options,
    longhelp : Option<String>
}

impl<'a> CoreOptions {
    pub fn new() -> Self {
        let mut ret = CoreOptions {
            options : getopts::Options::new(),
            longhelp : None
        };
        ret.options
            .optflag("", "help", "print usage information")
            .optflag("", "version", "print name and version number");
        ret
    }
    pub fn optopt(&mut self, short_name: &str, long_name: &str, desc: &str, hint: &str) -> &mut CoreOptions {
        self.options.optopt(short_name, long_name, desc, hint);
        self
    }
    pub fn optflag(&mut self, short_name: &str, long_name: &str, desc: &str) -> &mut CoreOptions {
        self.options.optflag(short_name, long_name, desc);
        self
    }
    pub fn help<T: Into<String>>(&mut self, longhelp : T) -> &mut CoreOptions {
        self.longhelp = Some(longhelp.into());
        self
    }
    pub fn usage(&self, summary : &str) -> String {
        self.options.usage(summary)
    }
    pub fn parse(&mut self, args : Vec<String>) -> getopts::Matches {
        let matches = match self.options.parse(&args[1..]) {
            Ok(m) => { Some(m) },
            Err(f) => {
                crash!(1, "{}", msg_invalid_input!(format!("{}", f)));
            }
        }.unwrap();
        if matches.opt_present("help") {
            exit!(match self.longhelp {
                Some(ref lhelp) => { print!("{}", lhelp); 0}
                None => 1
            });
        } else if matches.opt_present("version") {
            print!("{}", msg_version!());
            exit!(0);
        }
        matches
    }
}