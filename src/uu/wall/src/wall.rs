// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use clap::builder::ValueParser;
use clap::parser::ValuesRef;
use clap::{Arg, ArgAction, Command};
use jiff::Zoned;
use rustix::system::uname;
use std::env;
use std::ffi::OsString;
use std::io;
use std::io::prelude::*;
use std::string::FromUtf8Error;
use thiserror::Error;

use uucore::error::{UError, UResult};
use uucore::format_usage;
use uucore::utmpx::Utmpx;

use uucore::translate;
const STRING: &str = "string";
const OPT_GROUP: &str = "group";
const OPT_NOBANNER: &str = "nobanner";
const OPT_TIMEOUT: &str = "timeout";

#[derive(Error, Debug)]
enum WallError {
    #[error("{}", translate!("wall-error-stdin"))]
    Stdin(#[from] io::Error),
    #[error("{}", translate!("wall-encoding-error"))]
    VecToString(#[from] FromUtf8Error),
    #[error("{}", translate!("wall-error-osstring"))]
    ToStringError,
    #[error("{}", translate!("wall-error-mac-os-too-many-args"))]
    MacOsTooManyArgs,
}

impl UError for WallError {
    fn code(&self) -> i32 {
        1
    }
}

#[uucore::main(no_signals)]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args = args.skip(1).peekable();
    let matches = uucore::clap_localization::handle_clap_result(uu_app(), args)?;
    let message = get_message(matches.get_many(STRING).unwrap_or_default())?;
    let users = find_logged_users();
    write_to_terminals(message, users)?;
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new("wall")
        .version(uucore::crate_version!())
        .about(translate!("wall-about"))
        .override_usage(format_usage(&translate!("pwd-usage")))
        .arg(
            Arg::new(OPT_GROUP) // TODO(FEAT): Implement -g/--groups to target specific
                // users inside a group
                .short('g')
                .long(OPT_GROUP)
                .value_name("GROUP")
                .help(translate!("wall-help-group"))
                .num_args(1)
                .action(ArgAction::Append) // User can target more than one group
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new(OPT_NOBANNER) // TODO(FEAT): Implement -n/--nobanner to remove broadcasting
                // intro message
                .short('n')
                .long(OPT_NOBANNER)
                .action(ArgAction::SetTrue)
                .help(translate!("wall-help-nobanner")),
        )
        .arg(
            Arg::new(OPT_TIMEOUT) // TODO(FEAT): Implement -t --timeout to stop trying to print
                // after passed a delay
                .short('t')
                .long(OPT_TIMEOUT)
                .value_name("SECONDS")
                .help(translate!("wall-help-timeout"))
                .num_args(1),
        )
        .arg(
            Arg::new(STRING)
                .action(ArgAction::Append)
                .value_parser(ValueParser::os_string()),
        )
}

fn get_message(args: ValuesRef<OsString>) -> Result<String, WallError> {
    if args.len() == 0 {
        read_from_stdin()
    } else if args.len() == 1 {
        read_from_file(args.into_iter().next().unwrap())
    } else if cfg!(target_os = "macos") {
        Err(WallError::MacOsTooManyArgs)
    } else {
        concatenate_message(args)
    }
}

fn read_from_stdin() -> Result<String, WallError> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;
    let res = String::from_utf8(buffer)?;
    Ok(res)
}

fn read_from_file(file: &OsString) -> Result<String, WallError> {
    let mut buffer = Vec::new();
    let mut file = std::fs::File::open(file)?;
    file.read_to_end(&mut buffer)?;
    let res = String::from_utf8(buffer)?;
    Ok(res)
}

fn concatenate_message(args: ValuesRef<OsString>) -> Result<String, WallError> {
    let mut res = String::new();
    for arg in args {
        res.push_str(arg.to_str().ok_or(WallError::ToStringError)?);
        res.push(' ');
    }
    res.pop();
    Ok(res)
}

fn find_logged_users() -> Vec<OsString> {
    let mut res = Vec::<OsString>::new();
    for ut in Utmpx::iter_all_records() {
        if ut.is_user_process() {
            let mut tty_path = OsString::from("/dev/");
            tty_path.push(OsString::from(&ut.tty_device().clone()));
            res.push(tty_path);
        }
    }
    res
}

fn wall_intro_message() -> String {
    let user = "USER";
    let binding = uname();
    let hostname = binding.nodename().to_str().unwrap_or_default();

    let user = env::var_os(user).unwrap_or_default();
    // Fetch the TTY of the process calling wall (requires OS-specific calls or a wrapper function)
    let tty = "/dev/".to_owned() + &get_sender();

    let datetime = get_hour_and_date();
    #[cfg(target_os = "linux")]
    return format!(
        "\r\nBroadcast message from {}@{hostname} ({tty}) at {datetime} \r\n\r\n",
        user.to_string_lossy()
    );
    #[cfg(target_os = "macos")]
    return format!(
        "\r\nBroadcast message from {}@{hostname}\r\n\t({tty}) at {datetime}\r\n\r\n",
        user.to_string_lossy()
    );
}

fn write_to_terminals(message: String, users: Vec<OsString>) -> UResult<()> {
    let transmission = wall_intro_message() + &message + "\r\n\r\n";
    for user in users {
        let mut file = match std::fs::OpenOptions::new().write(true).open(user) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}: {e}", translate!("wall-error-open-terminal"));
                continue;
            }
        };
        write!(file, "{transmission}").map_err(|e| {
            eprintln!("{}:, {e}", translate!("wall-error-write-terminal"));
            WallError::Stdin(e)
        })?;
    }
    Ok(())
}

fn get_hour_and_date() -> String {
    #[cfg(target_os = "linux")]
    return Zoned::now().strftime("(%a %b %d %H:%M:%S %Y):").to_string();
    #[cfg(target_os = "macos")]
    return Zoned::now().strftime("%H:%M %Z...").to_string();
}

fn get_sender() -> String {
    rustix::termios::ttyname(io::stdin(), Vec::with_capacity(16))
        .map(|s| s.to_string_lossy().trim_start_matches("/dev/").to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {

    use crate::{OPT_GROUP, STRING};
    use crate::{find_logged_users, get_message, uu_app, write_to_terminals};
    use std::ffi::OsString;
    use std::process::{Command, Output};

    #[test]
    fn test_basic_clap_implementation() {
        let group = String::from("staff");
        let file = String::from("LICENSE");
        let command = vec!["wall", "-g", &group, &file];
        let matches = uucore::clap_localization::handle_clap_result(uu_app(), command)
            .expect("Error outside of test perimeter");
        assert!(matches.get_one::<String>(OPT_GROUP).unwrap() == &group);
        assert!(
            matches
                .get_one::<OsString>(STRING)
                .unwrap()
                .clone()
                .into_string()
                .unwrap()
                == file
        );
    }

    #[test]
    fn test_get_message_on_file() {
        let file = String::from("LICENSE");

        // wall does not print the content of the file in the stdout, it sends it to the tty(s)
        // Hence the use of cat to check if the get_message function can extract correctly the
        // file
        let mut command = Command::new("cat");
        command.arg(&file);
        let output: Output = command.output().expect("Failed to start 'cat' command");
        assert!(
            output.status.success(),
            "'cat' command exit with failure status"
        );
        let command_output =
            String::from_utf8(output.stdout).expect("Failed to convert 'cat'output");

        let command = vec!["wall", &file];
        let matches = uucore::clap_localization::handle_clap_result(uu_app(), command)
            .expect("External error");
        let pos_arg = matches.get_many(STRING).unwrap_or_default();
        let function_output = get_message(pos_arg).unwrap();
        assert_eq!(function_output, command_output);
    }

    #[test]
    fn test_get_message_on_stdin() {
        // for the moment test against cat is not implemented
        let command = vec!["wall"];
        let matches = uucore::clap_localization::handle_clap_result(uu_app(), command)
            .expect("External error");
        let pos_arg = matches.get_many(STRING).unwrap_or_default();
        let function_output = get_message(pos_arg).unwrap();
        assert_eq!(function_output, "Hello !\n");
    }

    #[test]
    fn test_arguments_as_message() {
        let command = vec!["wall", "Hello", "World", "!"];
        let matches = uucore::clap_localization::handle_clap_result(uu_app(), command)
            .expect("External error");
        let pos_arg = matches.get_many(STRING).unwrap_or_default();
        let function_output = get_message(pos_arg).unwrap();
        assert_eq!(function_output, "Hello World !");
    }

    #[test]
    fn test_found_connected_users() {
        let users = find_logged_users();
        assert_eq!(
            users,
            vec!(
                OsString::from("tty1"),
                OsString::from("tty2"),
                OsString::from("tty3")
            )
        );
    }

    #[test]
    fn test_print_to_terminals() {
        let users = find_logged_users();
        let _ = write_to_terminals(String::from("hello world!"), users);
        let _ = write_to_terminals(
            String::from("hello world!"),
            vec![OsString::from("/dev/tty1")],
        );
    }

    #[test]
    fn test_get_sender() {
        let sender = crate::get_sender();
        assert_eq!(sender, "pts/0");
    }
}
