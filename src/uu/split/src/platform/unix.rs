// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
use std::env;
use std::ffi::OsStr;
use std::io::Write;
use std::io::{BufWriter, Error, Result};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use uucore::error::USimpleError;
use uucore::fs;
use uucore::fs::FileInformation;
use uucore::show;
use uucore::translate;

/// A writer that writes to a `shell_process`' stdin
///
/// We use a shell process (not directly calling a sub-process) so we can forward the name of the
/// corresponding output file (xaa, xab, xac… ). This is the way it was implemented in GNU split.
struct FilterWriter {
    /// Running shell process
    shell_process: Child,
}

impl Write for FilterWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.shell_process
            .stdin
            .as_mut()
            .expect("failed to get shell stdin")
            .write(buf)
    }
    fn flush(&mut self) -> Result<()> {
        self.shell_process
            .stdin
            .as_mut()
            .expect("failed to get shell stdin")
            .flush()
    }
}

/// Have an environment variable set at a value during this lifetime
struct WithEnvVarSet {
    /// Env var key
    _previous_var_key: String,
    /// Previous value set to this key
    _previous_var_value: std::result::Result<String, env::VarError>,
}
impl WithEnvVarSet {
    /// Save previous value assigned to key, set key=value
    fn new(key: &str, value: &str) -> Self {
        let previous_env_value = env::var(key);
        unsafe {
            env::set_var(key, value);
        }
        Self {
            _previous_var_key: String::from(key),
            _previous_var_value: previous_env_value,
        }
    }
}

impl Drop for WithEnvVarSet {
    /// Restore previous value now that this is being dropped by context
    fn drop(&mut self) {
        if let Ok(ref prev_value) = self._previous_var_value {
            unsafe {
                env::set_var(&self._previous_var_key, prev_value);
            }
        } else {
            unsafe {
                env::remove_var(&self._previous_var_key);
            }
        }
    }
}
impl FilterWriter {
    /// Create a new filter running a command with $FILE pointing at the output name
    ///
    /// #Arguments
    ///
    /// * `command` - The shell command to execute
    /// * `filepath` - Path of the output file (forwarded to command as $FILE)
    fn new(command: &str, filepath: &str) -> Result<Self> {
        // set $FILE, save previous value (if there was one)
        let _with_env_var_set = WithEnvVarSet::new("FILE", filepath);

        let shell_process =
            Command::new(env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()))
                .arg("-c")
                .arg(command)
                .stdin(Stdio::piped())
                .spawn()?;

        Ok(Self { shell_process })
    }
}

impl Drop for FilterWriter {
    /// flush stdin, close it and wait on `shell_process` before dropping self
    fn drop(&mut self) {
        {
            // close stdin by dropping it
            let _stdin = self.shell_process.stdin.as_mut();
        }
        let exit_status = self
            .shell_process
            .wait()
            .expect("Couldn't wait for child process");
        if let Some(return_code) = exit_status.code() {
            if return_code != 0 {
                show!(USimpleError::new(
                    1,
                    translate!("split-error-shell-process-returned", "code" => return_code)
                ));
            }
        } else {
            show!(USimpleError::new(
                1,
                translate!("split-error-shell-process-terminated")
            ));
        }
    }
}

/// Instantiate either a file writer or a "write to shell process's stdin" writer
pub fn instantiate_current_writer(
    filter: Option<&str>,
    filename: &str,
    is_new: bool,
) -> Result<BufWriter<Box<dyn Write>>> {
    match filter {
        None => {
            let file = if is_new {
                // create new file
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(Path::new(&filename))
                    .map_err(|_| {
                        Error::other(
                            translate!("split-error-unable-to-open-file", "file" => filename),
                        )
                    })?
            } else {
                // re-open file that we previously created to append to it
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(Path::new(&filename))
                    .map_err(|_| {
                        Error::other(
                            translate!("split-error-unable-to-reopen-file", "file" => filename),
                        )
                    })?
            };
            Ok(BufWriter::new(Box::new(file) as Box<dyn Write>))
        }
        Some(filter_command) => Ok(BufWriter::new(Box::new(
            // spawn a shell command and write to it
            FilterWriter::new(filter_command, filename)?,
        ) as Box<dyn Write>)),
    }
}

pub fn paths_refer_to_same_file(p1: &OsStr, p2: &OsStr) -> bool {
    // We have to take symlinks and relative paths into account.
    let p1 = if p1 == "-" {
        FileInformation::from_file(&std::io::stdin())
    } else {
        FileInformation::from_path(Path::new(p1), true)
    };
    fs::infos_refer_to_same_file(p1, FileInformation::from_path(Path::new(p2), true))
}
