/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alex Lyon <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

#[macro_export]
macro_rules! executable(
    () => ({
        let module = module_path!();
        if &module[0..3] == "uu_" {
            &module[3..]
        } else {
            module
        }
    })
);

#[macro_export]
macro_rules! show_error(
    ($fd:expr, $($args:tt)+) => ({
        write!($fd.stderr, "{}: error: ", $fd.name)
            .and_then(|_| writeln!($fd.stderr, $($args)+))
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        eprint!("{}: warning: ", executable!());
        eprintln!($($args)+);
    })
);

#[macro_export]
macro_rules! show_info(
    ($($args:tt)+) => ({
        eprint!("{}: ", executable!());
        eprintln!($($args)+);
    })
);

#[macro_export]
macro_rules! disp_err(
    ($($args:tt)+) => ({
        eprint!("{}: ", executable!());
        eprintln!($($args)+);
        eprintln!("Try '{} --help' for more information.", executable!());
    })
);

#[macro_export]
macro_rules! return_if_err(
    ($exitcode:expr, $fd: expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => {
                show_error!($fd, "{}", f);
                return $exitcode;
            }
        }
    )
);

#[macro_export]
macro_rules! generate_from_impl(
    ($name:ident, $entry:ident, $from:path) => (
        impl From<$from> for $name {
            fn from(orig: $from) -> $name {
                $name::$entry(orig)
            }
        }
    )
);

#[macro_export]
macro_rules! xxxxxxxgenerate_error_type(
    ($name:ident, $($err:path, $exitcode:tt)*) => (
        #[derive(Debug, Fail)]
        #[fail(display = "{}", err)]
        pub struct $name {
            exitcode: i32,
            err: $crate::failure::Error
        }

        impl From<::std::io::Error> for $name {
            fn from(err: ::std::io::Error) -> $name {
                $name {
                    exitcode: if err.kind() == ::std::io::ErrorKind::BrokenPipe {
                        $crate::PIPE_EXITCODE
                    } else {
                        1
                    },
                    err: err.into()
                }
            }
        }

        impl From<$crate::failure::Error> for $name {
            fn from(err: $crate::failure::Error) -> $name {
                $name {
                    exitcode: 1,
                    err: err.into()
                }
            }
        }

        impl<T> From<$crate::failure::Context<T>> for $name {
            fn from(err: $crate::failure::Context<T>) -> $name {
                $name {
                    exitcode: 1,
                    err: err.into()
                }
            }
        }

        $(
            impl From<$err> for $name {
                fn from(err: $err) -> $name {
                    $name {
                        exitcode: generate_exitcode!(err, $exitcode),
                        err: err.into()
                    }
                }
            }
        )*

        impl $crate::UError for $name {
            fn code(&self) -> i32 { self.exitcode }
            fn error(self) -> $crate::failure::Error { self.err }
        }

        pub type Result<T> = ::std::result::Result<T, $name>;
    )
);

#[macro_export]
macro_rules! generate_exitcode(
    ($err:expr, _) => (
        $err.code()
    );
    ($err:expr, $exitcode:tt) => (
        $exitcode
    )
);

//-- message templates

//-- message templates : general

#[macro_export]
macro_rules! snippet_list_join_oxford {
    ($conjunction:expr, $valOne:expr, $valTwo:expr) => (
        format!("{}, {} {}", $valOne, $conjunction, $valTwo)
    );
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remainingVals:expr)*) => (
        format!("{}, {}", $valOne, snippet_list_join_inner!($conjunction, $valTwo $(, $remainingVals)*))
    );
}

#[macro_export]
macro_rules! snippet_list_join_or {
    ($valOne:expr, $valTwo:expr) => (
        format!("{} or {}", $valOne, $valTwo)
    );
    ($valOne:expr, $valTwo:expr $(, $remainingVals:expr)*) => (
        format!("{}, {}", $valOne, snippet_list_join_oxford!("or", $valTwo $(, $remainingVals)*))
    );
}

//-- message templates : invalid input

#[macro_export]
macro_rules! msg_invalid_input { ($reason: expr) => (
    format!("invalid input: {}", $reason) ); }

#[macro_export]
macro_rules! snippet_no_file_at_path { ($path:expr) => (
    format!("nonexistent path {}", $path) ); }

// -- message templates : invalid input : flag

#[macro_export]
macro_rules! msg_invalid_opt_use {
    ($about:expr, $flag:expr) => (
        msg_invalid_input!(format!("The '{}' option {}", $flag, $about))
    );
    ($about:expr, $longflag:expr, $shortflag:expr) => {
        msg_invalid_input!(format!("The '{}' ('{}') option {}", $longflag, $shortflag, $about))
    };
}

#[macro_export]
macro_rules! msg_opt_only_usable_if {
    ($clause:expr, $flag:expr) => (
        msg_invalid_opt_use!(format!("only usable if {}", $clause), $flag)
    );
    ($clause:expr, $longflag:expr, $shortflag:expr) => (
        msg_invalid_opt_use!(format!("only usable if {}", $clause), $longflag, $shortflag)
    );
}

#[macro_export]
macro_rules! msg_opt_invalid_should_be {
    ($expects:expr, $received:expr, $flag:expr) => (
        msg_invalid_opt_use!(format!("expects {}, but was provided {}", $expects, $received), $flag)
    );
    ($expects:expr, $received:expr, $longflag:expr, $shortflag:expr) => (
        msg_invalid_opt_use!(format!("expects {}, but was provided {}", $expects, $received), $longflag, $shortflag)
    );
}

// -- message templates : invalid input : args

#[macro_export]
macro_rules! msg_arg_invalid_value { ($expects:expr, $received:expr) => (
    msg_invalid_input!(format!("expects its argument to be {}, but was provided {}", $expects, $received)) ); }

#[macro_export]
macro_rules! msg_args_invalid_value {
    ($expects:expr, $received:expr) => (
        msg_invalid_input!(format!("expects its arguments to be {}, but was provided {}", $expects, $received))
    );
    ($msg:expr) => (
        msg_invalid_input!($msg)
    );
}

#[macro_export]
macro_rules! msg_args_nonexistent_file { ($received:expr) => (
    msg_args_invalid_value!("paths to files", snippet_no_file_at_path!($received)));}

#[macro_export]
macro_rules! msg_wrong_number_of_arguments {
    () => (
        msg_args_invalid_value!("wrong number of arguments")
    );
    ($min:expr, $max:expr) => (
        msg_args_invalid_value!(format!("expects {}-{} arguments", $min, $max))
    );
    ($exact:expr) => (
        if $exact == 1 {
            msg_args_invalid_value!("expects 1 argument")
        } else {
            msg_args_invalid_value!(format!("expects {} arguments", $exact))
        }
    );
}

// -- message templates : invalid input : input combinations

#[macro_export]
macro_rules! msg_expects_one_of {
    ($valOne:expr $(, $remainingVals:expr)*) => (
        msg_invalid_input!(format!("expects one of {}", snippet_list_join_or!($valOne $(, $remainingVals)*)))
    );
}

#[macro_export]
macro_rules! msg_expects_no_more_than_one_of {
    ($valOne:expr $(, $remainingVals:expr)*) => (
        msg_invalid_input!(format!("expects no more than one of {}", snippet_list_join_or!($valOne $(, $remainingVals)*))) ;
    );
}
