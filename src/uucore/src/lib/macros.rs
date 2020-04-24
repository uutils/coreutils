/*
 * This file is part of the uutils coreutils package.
 *
 * (c) Alex Lyon <arcterus@mail.com>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

// #[macro_export]
// macro_rules! main { ($($arg:tt)+) => ({
//     extern crate uu_arch;
//     use std::io::Write;
//     use uu_arch::uumain;

//     fn main() {
//         uucore::panic::install_sigpipe_hook();

//         let code = uumain(uucore::args().collect());
//         // Since stdout is line-buffered by default, we need to ensure any pending
//         // writes are flushed before exiting. Ideally, this should be enforced by
//         // each utility.
//         //
//         // See: https://github.com/rust-lang/rust/issues/23818
//         //
//         std::io::stdout().flush().expect("could not flush stdout");
//         std::process::exit(code);
//     }
// })}

// extern crate proc_macro;
// use proc_macro::TokenStream;
// #[proc_macro_attribute]
// pub fn hello(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let result = quote! {
//         fn main() {
//             uucore::panic::install_sigpipe_hook();
//             std::io::stdout().flush().expect("could not flush stdout");
//             std::process::exit(code);
//         }
//     };
// }

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
macro_rules! disp_err(
    ($($args:tt)+) => ({
        eprint!("{}: ", executable!());
        eprintln!($($args)+);
        eprintln!("Try '{} --help' for more information.", executable!());
    })
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
    ($about:expr, $longflag:expr, $shortflag:expr) => {
        msg_invalid_input!(format!(
            "The '{}' ('{}') option {}",
            $longflag, $shortflag, $about
        ))
    };
}

#[macro_export]
macro_rules! msg_opt_only_usable_if {
    ($clause:expr, $flag:expr) => {
        msg_invalid_opt_use!(format!("only usable if {}", $clause), $flag)
    };
    ($clause:expr, $longflag:expr, $shortflag:expr) => {
        msg_invalid_opt_use!(format!("only usable if {}", $clause), $longflag, $shortflag)
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
    ($expects:expr, $received:expr, $longflag:expr, $shortflag:expr) => {
        msg_invalid_opt_use!(
            format!("expects {}, but was provided {}", $expects, $received),
            $longflag,
            $shortflag
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
