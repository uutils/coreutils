pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_SUCCESS: i32 = 0;
pub const BUF_SIZE: usize = 65536;

#[inline(always)]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[inline(always)]
pub fn about() -> &'static str {
    "\
            Print the first 10 lines of each FILE to standard output.\n\
            With more than one FILE, precede each with a header giving the file name.\n\
            \n\
            With no FILE, or when FILE is -, read standard input.\n\
            \n\
            Mandatory arguments to long flags are mandatory for short flags too.\
            "
}

#[inline(always)]
pub fn usage() -> &'static str {
    "head [FLAG]... [FILE]..."
}

#[inline(always)]
pub fn bytes_name() -> &'static str {
    "BYTES"
}

#[inline(always)]
pub fn bytes_help() -> &'static str {
    "\
            print the first NUM bytes of each file;\n\
              with the leading '-', print all but the last\n\
              NUM bytes of each file\
            "
}

#[inline(always)]
pub fn lines_name() -> &'static str {
    "LINES"
}

#[inline(always)]
pub fn lines_help() -> &'static str {
    "\
            print the first NUM lines instead of the first 10;\n\
              with the leading '-', print all but the last\n\
              NUM lines of each file\
            "
}

#[inline(always)]
pub fn quiet_name() -> &'static str {
    "QUIET"
}

#[inline(always)]
pub fn quiet_help() -> &'static str {
    "never print headers giving file names"
}

#[inline(always)]
pub fn verbose_name() -> &'static str {
    "VERBOSE"
}

#[inline(always)]
pub fn verbose_help() -> &'static str {
    "always print headers giving file names"
}

#[inline(always)]
pub fn zero_name() -> &'static str {
    "ZERO"
}

#[inline(always)]
pub fn zero_help() -> &'static str {
    "line delimiter is NUL, not newline"
}

#[inline(always)]
pub fn files_name() -> &'static str {
    "FILE"
}
