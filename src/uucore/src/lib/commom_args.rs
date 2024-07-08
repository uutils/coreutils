//! Commom arguments for the utilities.

pub mod options {
    pub const HELP: &str = "help";
    pub mod verbosity {
        pub const CHANGES: &str = "changes";
        pub const QUIET: &str = "quiet";
        pub const SILENT: &str = "silent";
        pub const VERBOSE: &str = "verbose";
    }
    pub mod preserve_root {
        pub const PRESERVE: &str = "preserve-root";
        pub const NO_PRESERVE: &str = "no-preserve-root";
    }
    pub mod dereference {
        pub const DEREFERENCE: &str = "dereference";
        pub const NO_DEREFERENCE: &str = "no-dereference";
    }
    pub const FROM: &str = "from";
    pub const RECURSIVE: &str = "recursive";
    pub mod traverse {
        pub const TRAVERSE: &str = "H";
        pub const NO_TRAVERSE: &str = "P";
        pub const EVERY: &str = "L";
    }
    pub const REFERENCE: &str = "reference";
    pub const ARG_OWNER: &str = "OWNER";
    pub const ARG_GROUP: &str = "GROUP";
    pub const ARG_FILES: &str = "FILE";
}
