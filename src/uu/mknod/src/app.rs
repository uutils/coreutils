use clap::{crate_version, App, Arg};

const ABOUT: &str = "Create the special file NAME of the given TYPE.";
const USAGE: &str = "mknod [OPTION]... NAME TYPE [MAJOR MINOR]";
const LONG_HELP: &str = "Mandatory arguments to long options are mandatory for short options too.
-m, --mode=MODE    set file permission bits to MODE, not a=rw - umask
--help     display this help and exit
--version  output version information and exit

Both MAJOR and MINOR must be specified when TYPE is b, c, or u, and they
must be omitted when TYPE is p.  If MAJOR or MINOR begins with 0x or 0X,
it is interpreted as hexadecimal; otherwise, if it begins with 0, as octal;
otherwise, as decimal.  TYPE may be:

b      create a block (buffered) special file
c, u   create a character (unbuffered) special file
p      create a FIFO

NOTE: your shell may have its own version of mknod, which usually supersedes
the version described here.  Please refer to your shell's documentation
for details about the options it supports.
";

fn valid_type(tpe: String) -> Result<(), String> {
    // Only check the first character, to allow mnemonic usage like
    // 'mknod /dev/rst0 character 18 0'.
    tpe.chars()
        .next()
        .ok_or_else(|| "missing device type".to_string())
        .and_then(|first_char| {
            if vec!['b', 'c', 'u', 'p'].contains(&first_char) {
                Ok(())
            } else {
                Err(format!("invalid device type ‘{}’", tpe))
            }
        })
}

fn valid_u64(num: String) -> Result<(), String> {
    num.parse::<u64>().map(|_| ()).map_err(|_| num)
}

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .usage(USAGE)
        .after_help(LONG_HELP)
        .about(ABOUT)
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .value_name("MODE")
                .help("set file permission bits to MODE, not a=rw - umask"),
        )
        .arg(
            Arg::with_name("name")
                .value_name("NAME")
                .help("name of the new file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("type")
                .value_name("TYPE")
                .help("type of the new file (b, c, u or p)")
                .required(true)
                .validator(valid_type)
                .index(2),
        )
        .arg(
            Arg::with_name("major")
                .value_name("MAJOR")
                .help("major file type")
                .validator(valid_u64)
                .index(3),
        )
        .arg(
            Arg::with_name("minor")
                .value_name("MINOR")
                .help("minor file type")
                .validator(valid_u64)
                .index(4),
        )
}
