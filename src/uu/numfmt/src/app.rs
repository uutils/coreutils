use clap::{crate_version, App, AppSettings, Arg};

const ABOUT: &str = "Convert numbers from/to human-readable strings";
const LONG_HELP: &str = "UNIT options:
   none   no auto-scaling is done; suffixes will trigger an error

   auto   accept optional single/two letter suffix:

          1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

   si     accept optional single letter suffix:

          1K = 1000, 1M = 1000000, ...

   iec    accept optional single letter suffix:

          1K = 1024, 1M = 1048576, ...

   iec-i  accept optional two-letter suffix:

          1Ki = 1024, 1Mi = 1048576, ...

FIELDS supports cut(1) style field ranges:
  N    N'th field, counted from 1
  N-   from N'th field, to end of line
  N-M  from N'th to M'th field (inclusive)
  -M   from first to M'th field (inclusive)
  -    all fields
Multiple fields/ranges can be separated with commas
";

pub const DELIMITER: &str = "delimiter";
pub const FIELD: &str = "field";
pub const FIELD_DEFAULT: &str = "1";
pub const FROM: &str = "from";
pub const FROM_DEFAULT: &str = "none";
pub const HEADER: &str = "header";
pub const HEADER_DEFAULT: &str = "1";
pub const NUMBER: &str = "NUMBER";
pub const PADDING: &str = "padding";
pub const TO: &str = "to";
pub const TO_DEFAULT: &str = "none";

pub fn get_app(app_name: &str) -> App {
    App::new(app_name)
        .version(crate_version!())
        .about(ABOUT)
        .after_help(LONG_HELP)
        .setting(AppSettings::AllowNegativeNumbers)
        .arg(
            Arg::with_name(DELIMITER)
                .short("d")
                .long(DELIMITER)
                .value_name("X")
                .help("use X instead of whitespace for field delimiter"),
        )
        .arg(
            Arg::with_name(FIELD)
                .long(FIELD)
                .help("replace the numbers in these input fields (default=1) see FIELDS below")
                .value_name("FIELDS")
                .default_value(FIELD_DEFAULT),
        )
        .arg(
            Arg::with_name(FROM)
                .long(FROM)
                .help("auto-scale input numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(FROM_DEFAULT),
        )
        .arg(
            Arg::with_name(TO)
                .long(TO)
                .help("auto-scale output numbers to UNITs; see UNIT below")
                .value_name("UNIT")
                .default_value(TO_DEFAULT),
        )
        .arg(
            Arg::with_name(PADDING)
                .long(PADDING)
                .help(
                    "pad the output to N characters; positive N will \
                 right-align; negative N will left-align; padding is \
                 ignored if the output is wider than N; the default is \
                 to automatically pad if a whitespace is found",
                )
                .value_name("N"),
        )
        .arg(
            Arg::with_name(HEADER)
                .long(HEADER)
                .help(
                    "print (without converting) the first N header lines; \
                 N defaults to 1 if not specified",
                )
                .value_name("N")
                .default_value(HEADER_DEFAULT)
                .hide_default_value(true),
        )
        .arg(Arg::with_name(NUMBER).hidden(true).multiple(true))
}
