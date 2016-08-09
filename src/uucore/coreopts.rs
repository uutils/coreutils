extern crate getopts;
use std::io::Write;

pub struct CoreOptions {
    pub options : getopts::Options,
    pkgname: &'static str,
    longhelp : Option<String>
}

impl<'a> CoreOptions {
    pub fn new(name: &'static str) -> Self {
        let mut ret = CoreOptions {
            options : getopts::Options::new(),
            pkgname: name,
            longhelp: None
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
                eprintln!("{}: {}", self.pkgname, f);
                eprintln!("Try '{} --help' for more information.", self.pkgname);
                exit!(1)
            }
        }.unwrap();
        if matches.opt_present("help") {
            exit!(match self.longhelp {
                Some(ref lhelp) => { println!("{}", lhelp); 0}
                None => 1
            });
        } else if matches.opt_present("version") {
            println!("{} {}", self.pkgname, env!("CARGO_PKG_VERSION"));
            exit!(0);
        }
        matches
    }
}
