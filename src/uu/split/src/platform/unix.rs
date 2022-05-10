use std::env;
use std::io::Write;
use std::io::{BufWriter, Result};
use std::process::{Child, Command, Stdio};
use uucore::crash;

/// A writer that writes to a shell_process' stdin
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
        env::set_var(key, value);
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
            env::set_var(&self._previous_var_key, &prev_value);
        } else {
            env::remove_var(&self._previous_var_key);
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
    fn new(command: &str, filepath: &str) -> Self {
        // set $FILE, save previous value (if there was one)
        let _with_env_var_set = WithEnvVarSet::new("FILE", filepath);

        let shell_process =
            Command::new(env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()))
                .arg("-c")
                .arg(command)
                .stdin(Stdio::piped())
                .spawn()
                .expect("Couldn't spawn filter command");

        Self { shell_process }
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
                crash!(1, "Shell process returned {}", return_code);
            }
        } else {
            crash!(1, "Shell process terminated by signal")
        }
    }
}

/// Instantiate either a file writer or a "write to shell process's stdin" writer
pub fn instantiate_current_writer(
    filter: &Option<String>,
    filename: &str,
) -> BufWriter<Box<dyn Write>> {
    match filter {
        None => BufWriter::new(Box::new(
            // write to the next file
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(std::path::Path::new(&filename))
                .unwrap(),
        ) as Box<dyn Write>),
        Some(ref filter_command) => BufWriter::new(Box::new(
            // spawn a shell command and write to it
            FilterWriter::new(filter_command, filename),
        ) as Box<dyn Write>),
    }
}
