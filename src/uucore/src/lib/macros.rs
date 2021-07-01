// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

/// Get the executable path (as `OsString`).
#[macro_export]
macro_rules! executable_os(
    () => ({
        &std::env::args_os().next().unwrap()
    })
);

/// Get the executable path (as `String`; lossless).
#[macro_export]
macro_rules! executable(
    () => ({
        let exe = match $crate::executable_os!().to_str() {
            // * UTF-8
            Some(s) => s.to_string(),
            // * "lossless" debug format if `executable_os!()` is not well-formed UTF-8
            None => format!("{:?}", $crate::executable_os!())
        };
        &exe.to_owned()
    })
);

/// Get the executable name.
#[macro_export]
macro_rules! executable_name(
    () => ({
        &std::path::Path::new($crate::executable_os!()).file_stem().unwrap().to_string_lossy()
    })
);

/// Derive the utility name.
#[macro_export]
macro_rules! util_name(
    () => ({
        let crate_name = env!("CARGO_PKG_NAME");
        if crate_name.starts_with("uu_") {
            &crate_name[3..]
        } else {
            &crate_name
        }
    })
);

//====

#[macro_export]
macro_rules! show(
    ($err:expr) => ({
        let e = $err;
        uucore::error::set_exit_code(e.code());
        eprintln!("{}: {}", $crate::executable_name!(), e);
    })
);

#[macro_export]
macro_rules! show_if_err(
    ($res:expr) => ({
        if let Err(e) = $res {
            show!(e);
        }
    })
);

/// Show an error to stderr in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::executable_name!());
        eprintln!($($args)+);
    })
);

/// Show a warning to stderr in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_error_custom_description (
    ($err:expr,$($args:tt)+) => ({
        eprint!("{}: {}: ", $crate::executable_name!(), $err);
        eprintln!($($args)+);
    })
);

#[macro_export]
macro_rules! show_warning(
    ($($args:tt)+) => ({
        eprint!("{}: warning: ", $crate::executable_name!());
        eprintln!($($args)+);
    })
);

/// Show a bad invocation help message in a similar style to GNU coreutils.
#[macro_export]
macro_rules! show_usage_error(
    ($($args:tt)+) => ({
        eprint!("{}: ", $crate::executable_name!());
        eprintln!($($args)+);
        eprintln!("Try `{:?} --help` for more information.", $crate::executable!());
    })
);

//====

/// Calls `exit()` with the provided exit code.
#[macro_export]
macro_rules! exit(
    ($exit_code:expr) => ({
        ::std::process::exit($exit_code)
    })
);

/// Display the provided error message, then `exit()` with the provided exit code
#[macro_export]
macro_rules! crash(
    ($exit_code:expr, $($args:tt)+) => ({
        $crate::show_error!($($args)+);
        $crate::exit!($exit_code)
    })
);

/// Unwraps the Result. Instead of panicking, it exists the program with the
/// provided exit code.
#[macro_export]
macro_rules! crash_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => $crate::crash!($exit_code, "{}", f),
        }
    )
);

//====

/// Unwraps the Result. Instead of panicking, it shows the error and then
/// returns from the function with the provided exit code.
/// Assumes the current function returns an i32 value.
#[macro_export]
macro_rules! return_if_err(
    ($exit_code:expr, $exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => {
                $crate::show_error!("{}", f);
                return $exit_code;
            }
        }
    )
);

//====

#[macro_export]
macro_rules! safe_write(
    ($fd:expr, $($args:tt)+) => (
        match write!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!("{}", f)
        }
    )
);

#[macro_export]
macro_rules! safe_writeln(
    ($fd:expr, $($args:tt)+) => (
        match writeln!($fd, $($args)+) {
            Ok(_) => {}
            Err(f) => panic!("{}", f)
        }
    )
);

/// Unwraps the Result. Instead of panicking, it exists the program with exit
/// code 1.
#[macro_export]
macro_rules! safe_unwrap(
    ($exp:expr) => (
        match $exp {
            Ok(m) => m,
            Err(f) => $crate::crash!(1, "{}", f.to_string())
        }
    )
);

//-- message templates

//-- message templates : (join utility sub-macros)

#[macro_export]
macro_rules! snippet_list_join_oxford_comma {
    ($conjunction:expr, $valOne:expr, $valTwo:expr) => (
        format!("{}, {} {}", $valOne, $conjunction, $valTwo)
    );
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, $crate::snippet_list_join_oxford_comma!($conjunction, $valTwo $(, $remaining_values)*))
    );
}

#[macro_export]
macro_rules! snippet_list_join {
    ($conjunction:expr, $valOne:expr, $valTwo:expr) => (
        format!("{} {} {}", $valOne, $conjunction, $valTwo)
    );
    ($conjunction:expr, $valOne:expr, $valTwo:expr $(, $remaining_values:expr)*) => (
        format!("{}, {}", $valOne, $crate::snippet_list_join_oxford_comma!($conjunction, $valTwo $(, $remaining_values)*))
    );
}

//-- message templates : invalid input

#[macro_export]
macro_rules! msg_invalid_input {
    ($reason: expr) => {
        format!("invalid input: {}", $reason)
    };
}

// -- message templates : invalid input : flag

#[macro_export]
macro_rules! msg_invalid_opt_use {
    ($about:expr, $flag:expr) => {
        $crate::msg_invalid_input!(format!("The '{}' option {}", $flag, $about))
    };
    ($about:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_input!(format!(
            "The '{}' ('{}') option {}",
            $long_flag, $short_flag, $about
        ))
    };
}

#[macro_export]
macro_rules! msg_opt_only_usable_if {
    ($clause:expr, $flag:expr) => {
        $crate::msg_invalid_opt_use!(format!("only usable if {}", $clause), $flag)
    };
    ($clause:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("only usable if {}", $clause),
            $long_flag,
            $short_flag
        )
    };
}

#[macro_export]
macro_rules! msg_opt_invalid_should_be {
    ($expects:expr, $received:expr, $flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $flag
        )
    };
    ($expects:expr, $received:expr, $long_flag:expr, $short_flag:expr) => {
        $crate::msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $long_flag,
            $short_flag
        )
    };
}

// -- message templates : invalid input : input combinations

#[macro_export]
macro_rules! msg_expects_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        $crate::msg_invalid_input!(format!("expects one of {}", $crate::snippet_list_join!("or", $valOne $(, $remaining_values)*)))
    );
}

#[macro_export]
macro_rules! msg_expects_no_more_than_one_of {
    ($valOne:expr $(, $remaining_values:expr)*) => (
        $crate::msg_invalid_input!(format!("expects no more than one of {}", $crate::snippet_list_join!("or", $valOne $(, $remaining_values)*))) ;
    );
}
