// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

#[macro_export]
macro_rules! executable(
    () => ({
        let module = module_path!();
        let module = module.split("::").next().unwrap_or(module);
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
        eprint!("{}: error: ", executable!());
        eprintln!($($args)+);
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
macro_rules! show_usage_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", executable!());
        eprintln!($($args)+);
        eprintln!("Try '{} --help' for more information.", executable!());
    })
);

#[macro_export]
macro_rules! crash(
    ($exit_code:expr, $($args:tt)+) => ({
        show_error!($($args)+);
        ::std::process::exit($exit_code)
    })
);

#[macro_export]
macro_rules! exit(
    ($exit_code:expr) => ({
        ::std::process::exit($exit_code)
    })
);

#[macro_export]
macro_rules! crash_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => crash!($exit_code, "{}", f),
        }
    )
);

#[macro_export]
macro_rules! return_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => {
                show_error!("{}", f);
                return $exit_code;
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
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, snippet_list_join_inner!($conjunction, $valTwo $(, $remaining_values)*))
    );
}

#[macro_export]
macro_rules! snippet_list_join_or {
    ($valOne:expr, $valTwo:expr) => (
        format!("{} or {}", $valOne, $valTwo)
    );
    ($valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, snippet_list_join_oxford!("or", $valTwo $(, $remaining_values)*))
    );
}

//-- message templates : invalid input

#[macro_export]
macro_rules! msg_invalid_input {
    ($reason: expr) => {
        format!("invalid input: {}", $reason)
    };
}

#[macro_export]
macro_rules! snippet_no_file_at_path {
    ($path:expr) => {
        format!("nonexistent path {}", $path)
    };
}

// -- message templates : invalid input : flag

#[macro_export]
macro_rules! msg_invalid_opt_use {
    ($about:expr, $flag:expr) => {
        msg_invalid_input!(format!("The '{}' option {}", $flag, $about))
    };
    ($about:expr, $long_flag:expr, $short_flag:expr) => {
        msg_invalid_input!(format!(
            "The '{}' ('{}') option {}",
            $long_flag, $short_flag, $about
        ))
    };
}

#[macro_export]
macro_rules! msg_opt_only_usable_if {
    ($clause:expr, $flag:expr) => {
        msg_invalid_opt_use!(format!("only usable if {}", $clause), $flag)
    };
    ($clause:expr, $long_flag:expr, $short_flag:expr) => {
        msg_invalid_opt_use!(
            format!("only usable if {}", $clause),
            $long_flag,
            $short_flag
        )
    };
}

#[macro_export]
macro_rules! msg_opt_invalid_should_be {
    ($expects:expr, $received:expr, $flag:expr) => {
        msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $flag
        )
    };
    ($expects:expr, $received:expr, $long_flag:expr, $short_flag:expr) => {
        msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $long_flag,
            $short_flag
        )
    };
}

// -- message templates : invalid input : args

#[macro_export]
macro_rules! msg_arg_invalid_value {
    ($expects:expr, $received:expr) => {
        msg_invalid_input!(format!(
            "expects its argument to be {}, but was provided {}",
            $expects, $received
        ))
    };
}

#[macro_export]
macro_rules! msg_args_invalid_value {
    ($expects:expr, $received:expr) => {
        msg_invalid_input!(format!(
            "expects its arguments to be {}, but was provided {}",
            $expects, $received
        ))
    };
    ($msg:expr) => {
        msg_invalid_input!($msg)
    };
}

#[macro_export]
macro_rules! msg_args_nonexistent_file {
    ($received:expr) => {
        msg_args_invalid_value!("paths to files", snippet_no_file_at_path!($received))
    };
}

#[macro_export]
macro_rules! msg_wrong_number_of_arguments {
    () => {
        msg_args_invalid_value!("wrong number of arguments")
    };
    ($min:expr, $max:expr) => {
        msg_args_invalid_value!(format!("expects {}-{} arguments", $min, $max))
    };
    ($exact:expr) => {
        if $exact == 1 {
            msg_args_invalid_value!("expects 1 argument")
        } else {
            msg_args_invalid_value!(format!("expects {} arguments", $exact))
        }
    };
}

// -- message templates : invalid input : input combinations

#[macro_export]
macro_rules! msg_expects_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        msg_invalid_input!(format!("expects one of {}", snippet_list_join_or!($valOne $(, $remaining_values)*)))
    );
}

#[macro_export]
macro_rules! msg_expects_no_more_than_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        msg_invalid_input!(format!("expects no more than one of {}", snippet_list_join_or!($valOne $(, $remaining_values)*))) ;
    );
}
