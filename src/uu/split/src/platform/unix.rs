// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.

use crate::platform::Writer;
use crate::platform::output_file::open_output_file;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::Result;
use std::io::Write;
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
pub struct FilterWriter {
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
    previous_var_key: String,
    /// Previous value set to this key
    previous_var_value: Option<OsString>,
}
impl WithEnvVarSet {
    /// Save previous value assigned to key, set key=value
    fn new(key: &str, value: &OsStr) -> Self {
        let previous_env_value = env::var_os(key);
        unsafe {
            env::set_var(key, value);
        }
        Self {
            previous_var_key: String::from(key),
            previous_var_value: previous_env_value,
        }
    }
}

impl Drop for WithEnvVarSet {
    /// Restore previous value now that this is being dropped by context
    fn drop(&mut self) {
        if let Some(prev_value) = &self.previous_var_value {
            unsafe {
                env::set_var(&self.previous_var_key, prev_value);
            }
        } else {
            unsafe {
                env::remove_var(&self.previous_var_key);
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
    fn new(command: &str, filepath: &OsStr) -> Result<Self> {
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
    input: &OsStr,
    filename: &OsStr,
    is_new: bool,
) -> Result<Writer> {
    match filter {
        None => Ok(Writer::File(open_output_file(input, filename, is_new)?)),
        Some(filter_command) => Ok(
            // spawn a shell command and write to it
            Writer::Filter(FilterWriter::new(filter_command, filename)?),
        ),
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

#[cfg(test)]
mod tests {
    use super::instantiate_current_writer;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reopened_writer_rejects_symlink_swapped_to_input() {
        let tmp = std::env::temp_dir().join(format!(
            "uutils-split-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&tmp).expect("failed to create temp dir");

        let input = tmp.join("input.txt");
        let output = tmp.join("xaa");
        fs::write(&input, b"input\n").expect("failed to write input");
        symlink(&input, &output).expect("failed to create output symlink");

        let Err(err) =
            instantiate_current_writer(None, input.as_os_str(), output.as_os_str(), false)
        else {
            panic!("re-opened writer should reject swapped symlink");
        };
        assert!(
            err.to_string()
                .contains("split-error-would-overwrite-input"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(&tmp).expect("failed to cleanup temp dir");
    }
}
