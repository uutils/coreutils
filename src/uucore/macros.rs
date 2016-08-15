/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Arcterus <arcterus@mail.com>
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
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: error: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: warning: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! show_info(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
    })
);

#[macro_export]
macro_rules! disp_err(
    ($($args:tt)+) => ({
        pipe_write!(&mut ::std::io::stderr(), "{}: ", executable!());
        pipe_writeln!(&mut ::std::io::stderr(), $($args)+);
        pipe_writeln!(&mut ::std::io::stderr(), "Try '{} --help' for more information.", executable!());
    })
);

#[macro_export]
macro_rules! eprint(
    ($($args:tt)+) => (pipe_write!(&mut ::std::io::stderr(), $($args)+))
);

#[macro_export]
macro_rules! eprintln(
    ($($args:tt)+) => (pipe_writeln!(&mut ::std::io::stderr(), $($args)+))
);

#[macro_export]
macro_rules! crash(
    ($exitcode:expr, $($args:tt)+) => ({
        show_error!($($args)+);
        ::std::process::exit($exitcode)
    })
);

#[macro_export]
macro_rules! exit(
    ($exitcode:expr) => ({
        ::std::process::exit($exitcode)
    })
);

#[macro_export]
macro_rules! crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!($exitcode, "{}", f),
        }
    )
);

#[macro_export]
macro_rules! pipe_crash_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(_) => (),
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    ()
                } else {
                    crash!($exitcode, "{}", f)
                }
            },
        }
    )
);

#[macro_export]
macro_rules! return_if_err(
    ($exitcode:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => {
                show_error!("{}", f);
                return $exitcode;
            }
        }
    )
);

// XXX: should the pipe_* macros return an Err just to show the write failed?

#[macro_export]
macro_rules! pipe_print(
    ($($args:tt)+) => (
        match write!(&mut ::std::io::stdout(), $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_println(
    ($($args:tt)+) => (
        match writeln!(&mut ::std::io::stdout(), $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_write(
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_writeln(
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! pipe_flush(
    () => (
        match ::std::io::stdout().flush() {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    );
    ($fd:expr) => (
        match $fd.flush() {
            Ok(_) => true,
            Err(f) => {
                if f.kind() == ::std::io::ErrorKind::BrokenPipe {
                    false
                } else {
                    panic!("{}", f)
                }
            }
        }
    )
);

#[macro_export]
macro_rules! safe_write(
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!(f.to_string())
        }
    )
);

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!(f.to_string())
        }
    )
);

#[macro_export]
macro_rules! safe_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!(1, "{}", f.to_string())
        }
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
