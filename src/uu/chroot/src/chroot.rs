// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

// spell-checker:ignore (ToDO) NEWROOT Userspec pstatus chdir
mod error;

use crate::error::ChrootError;
use clap::{crate_version, Arg, ArgAction, Command};
use std::ffi::CString;
use std::io::Error;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::process;
use uucore::error::{set_exit_code, UClapError, UResult, UUsageError};
use uucore::fs::{canonicalize, MissingHandling, ResolveMode};
use uucore::libc::{self, chroot, setgid, setgroups, setuid};
use uucore::{entries, format_usage, help_about, help_usage, show};

static ABOUT: &str = help_about!("chroot.md");
static USAGE: &str = help_usage!("chroot.md");

mod options {
    pub const NEWROOT: &str = "newroot";
    pub const GROUPS: &str = "groups";
    pub const USERSPEC: &str = "userspec";
    pub const COMMAND: &str = "command";
    pub const SKIP_CHDIR: &str = "skip-chdir";
}

/// A user and group specification, where each is optional.
enum UserSpec {
    NeitherGroupNorUser,
    UserOnly(String),
    GroupOnly(String),
    UserAndGroup(String, String),
}

struct Options {
    /// Path to the new root directory.
    newroot: PathBuf,
    /// Whether to change to the new root directory.
    skip_chdir: bool,
    /// List of groups under which the command will be run.
    groups: Vec<String>,
    /// The user and group (each optional) under which the command will be run.
    userspec: Option<UserSpec>,
}

/// Parse a user and group from the argument to `--userspec`.
///
/// The `spec` must be of the form `[USER][:[GROUP]]`, otherwise an
/// error is returned.
fn parse_userspec(spec: &str) -> UResult<UserSpec> {
    match &spec.splitn(2, ':').collect::<Vec<&str>>()[..] {
        // ""
        [""] => Ok(UserSpec::NeitherGroupNorUser),
        // "usr"
        [usr] => Ok(UserSpec::UserOnly(usr.to_string())),
        // ":"
        ["", ""] => Ok(UserSpec::NeitherGroupNorUser),
        // ":grp"
        ["", grp] => Ok(UserSpec::GroupOnly(grp.to_string())),
        // "usr:"
        [usr, ""] => Ok(UserSpec::UserOnly(usr.to_string())),
        // "usr:grp"
        [usr, grp] => Ok(UserSpec::UserAndGroup(usr.to_string(), grp.to_string())),
        // everything else
        _ => Err(ChrootError::InvalidUserspec(spec.to_string()).into()),
    }
}

// Pre-condition: `list_str` is non-empty.
fn parse_group_list(list_str: &str) -> Result<Vec<String>, ChrootError> {
    let split: Vec<&str> = list_str.split(",").collect();
    if split.len() == 1 {
        let name = split[0].trim();
        if name.is_empty() {
            // --groups=" "
            // chroot: invalid group ‘ ’
            Err(ChrootError::InvalidGroup(name.to_string()))
        } else {
            // --groups="blah"
            Ok(vec![name.to_string()])
        }
    } else if split.iter().all(|s| s.is_empty()) {
        // --groups=","
        // chroot: invalid group list ‘,’
        Err(ChrootError::InvalidGroupList(list_str.to_string()))
    } else {
        let mut result = vec![];
        let mut err = false;
        for name in split {
            let trimmed_name = name.trim();
            if trimmed_name.is_empty() {
                if name.is_empty() {
                    // --groups=","
                    continue;
                } else {
                    // --groups=", "
                    // chroot: invalid group ‘ ’
                    show!(ChrootError::InvalidGroup(name.to_string()));
                    err = true;
                }
            } else {
                // TODO Figure out a better condition here.
                if trimmed_name.starts_with(char::is_numeric)
                    && trimmed_name.ends_with(|c: char| !c.is_numeric())
                {
                    // --groups="0trail"
                    // chroot: invalid group ‘0trail’
                    show!(ChrootError::InvalidGroup(name.to_string()));
                    err = true;
                } else {
                    result.push(trimmed_name.to_string());
                }
            }
        }
        if err {
            Err(ChrootError::GroupsParsingFailed)
        } else {
            Ok(result)
        }
    }
}

impl Options {
    /// Parse parameters from the command-line arguments.
    fn from(matches: &clap::ArgMatches) -> UResult<Self> {
        let newroot = match matches.get_one::<String>(options::NEWROOT) {
            Some(v) => Path::new(v).to_path_buf(),
            None => return Err(ChrootError::MissingNewRoot.into()),
        };
        let groups = match matches.get_one::<String>(options::GROUPS) {
            None => vec![],
            Some(s) => {
                if s.is_empty() {
                    vec![]
                } else {
                    parse_group_list(s)?
                }
            }
        };
        let skip_chdir = matches.get_flag(options::SKIP_CHDIR);
        let userspec = match matches.get_one::<String>(options::USERSPEC) {
            None => None,
            Some(s) => Some(parse_userspec(s)?),
        };
        Ok(Self {
            newroot,
            skip_chdir,
            groups,
            userspec,
        })
    }
}

#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let matches = uu_app().try_get_matches_from(args).with_exit_code(125)?;

    let default_shell: &'static str = "/bin/sh";
    let default_option: &'static str = "-i";
    let user_shell = std::env::var("SHELL");

    let options = Options::from(&matches)?;

    // We are resolving the path in case it is a symlink or /. or /../
    if options.skip_chdir
        && canonicalize(
            &options.newroot,
            MissingHandling::Normal,
            ResolveMode::Logical,
        )
        .unwrap()
        .to_str()
            != Some("/")
    {
        return Err(UUsageError::new(
            125,
            "option --skip-chdir only permitted if NEWROOT is old '/'",
        ));
    }

    if !options.newroot.is_dir() {
        return Err(ChrootError::NoSuchDirectory(format!("{}", options.newroot.display())).into());
    }

    let commands = match matches.get_many::<String>(options::COMMAND) {
        Some(v) => v.map(|s| s.as_str()).collect(),
        None => vec![],
    };

    // TODO: refactor the args and command matching
    // See: https://github.com/uutils/coreutils/pull/2365#discussion_r647849967
    let command: Vec<&str> = match commands.len() {
        0 => {
            let shell: &str = match user_shell {
                Err(_) => default_shell,
                Ok(ref s) => s.as_ref(),
            };
            vec![shell, default_option]
        }
        _ => commands,
    };

    assert!(!command.is_empty());
    let chroot_command = command[0];
    let chroot_args = &command[1..];

    // NOTE: Tests can only trigger code beyond this point if they're invoked with root permissions
    set_context(&options)?;

    let pstatus = match process::Command::new(chroot_command)
        .args(chroot_args)
        .status()
    {
        Ok(status) => status,
        Err(e) => {
            return Err(if e.kind() == std::io::ErrorKind::NotFound {
                ChrootError::CommandNotFound(command[0].to_string(), e)
            } else {
                ChrootError::CommandFailed(command[0].to_string(), e)
            }
            .into())
        }
    };

    let code = if pstatus.success() {
        0
    } else {
        pstatus.code().unwrap_or(-1)
    };
    set_exit_code(code);
    Ok(())
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(crate_version!())
        .about(ABOUT)
        .override_usage(format_usage(USAGE))
        .infer_long_args(true)
        .trailing_var_arg(true)
        .arg(
            Arg::new(options::NEWROOT)
                .value_hint(clap::ValueHint::DirPath)
                .hide(true)
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new(options::GROUPS)
                .short('G')
                .long(options::GROUPS)
                .help("Comma-separated list of groups to switch to")
                .value_name("GROUP1,GROUP2..."),
        )
        .arg(
            Arg::new(options::USERSPEC)
                .long(options::USERSPEC)
                .help(
                    "Colon-separated user and group to switch to. \
                     Same as -u USER -g GROUP. \
                     Userspec has higher preference than -u and/or -g",
                )
                .value_name("USER:GROUP"),
        )
        .arg(
            Arg::new(options::SKIP_CHDIR)
                .long(options::SKIP_CHDIR)
                .help(
                    "Use this option to not change the working directory \
                    to / after changing the root directory to newroot, \
                    i.e., inside the chroot.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new(options::COMMAND)
                .action(ArgAction::Append)
                .value_hint(clap::ValueHint::CommandName)
                .hide(true)
                .index(2),
        )
}

fn set_context(options: &Options) -> UResult<()> {
    enter_chroot(&options.newroot, options.skip_chdir)?;
    set_groups_from_str(&options.groups)?;
    match &options.userspec {
        None | Some(UserSpec::NeitherGroupNorUser) => {}
        Some(UserSpec::UserOnly(user)) => set_user(user)?,
        Some(UserSpec::GroupOnly(group)) => set_main_group(group)?,
        Some(UserSpec::UserAndGroup(user, group)) => {
            set_main_group(group)?;
            set_user(user)?;
        }
    }
    Ok(())
}

fn enter_chroot(root: &Path, skip_chdir: bool) -> UResult<()> {
    let err = unsafe {
        chroot(
            CString::new(root.as_os_str().as_bytes().to_vec())
                .unwrap()
                .as_bytes_with_nul()
                .as_ptr() as *const libc::c_char,
        )
    };

    if err == 0 {
        if !skip_chdir {
            std::env::set_current_dir(root).unwrap();
        }
        Ok(())
    } else {
        Err(ChrootError::CannotEnter(format!("{}", root.display()), Error::last_os_error()).into())
    }
}

fn set_main_group(group: &str) -> UResult<()> {
    if !group.is_empty() {
        let group_id = match entries::grp2gid(group) {
            Ok(g) => g,
            _ => return Err(ChrootError::NoSuchGroup.into()),
        };
        let err = unsafe { setgid(group_id) };
        if err != 0 {
            return Err(
                ChrootError::SetGidFailed(group_id.to_string(), Error::last_os_error()).into(),
            );
        }
    }
    Ok(())
}

#[cfg(any(target_vendor = "apple", target_os = "freebsd", target_os = "openbsd"))]
fn set_groups(groups: &[libc::gid_t]) -> libc::c_int {
    unsafe { setgroups(groups.len() as libc::c_int, groups.as_ptr()) }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_groups(groups: &[libc::gid_t]) -> libc::c_int {
    unsafe { setgroups(groups.len() as libc::size_t, groups.as_ptr()) }
}

fn set_groups_from_str(groups: &[String]) -> UResult<()> {
    if !groups.is_empty() {
        let mut groups_vec = vec![];
        for group in groups {
            let gid = match entries::grp2gid(group) {
                Ok(g) => g,
                Err(_) => return Err(ChrootError::NoSuchGroup.into()),
            };
            groups_vec.push(gid);
        }
        let err = set_groups(&groups_vec);
        if err != 0 {
            return Err(ChrootError::SetGroupsFailed(Error::last_os_error()).into());
        }
    }
    Ok(())
}

fn set_user(user: &str) -> UResult<()> {
    if !user.is_empty() {
        let user_id = entries::usr2uid(user).map_err(|_| ChrootError::NoSuchUser)?;
        let err = unsafe { setuid(user_id as libc::uid_t) };
        if err != 0 {
            return Err(
                ChrootError::SetUserFailed(user.to_string(), Error::last_os_error()).into(),
            );
        }
    }
    Ok(())
}
