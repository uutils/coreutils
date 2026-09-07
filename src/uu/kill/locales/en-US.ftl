kill-about = Send signal to processes or list information about signals.
kill-usage = kill [OPTIONS]... PID...
kill-after-help-windows = Windows notes:
  Signalled processes are force-terminated (Windows has no signal delivery);
  their exit status is 128 plus the signal number. Negative PIDs (another
  process's group) and STOP are not supported. Permissions come from your
  current token, with SeDebugPrivilege enabled when it is held, so run
  elevated to reach processes a standard token cannot signal. Protected
  (anti-malware) processes cannot be terminated at all.

  PID 0 targets the Job object kill runs in, the closest Windows analog of a
  process group. Every process in that job and in its child jobs is signalled,
  kill itself last, so kill dies with the group. Outside a job, PID 0 signals
  only kill itself.

  Beware: a Job object is usually not yours. Terminals, IDEs, Docker, CI
  agents and Windows' own Program Compatibility Assistant all run what they
  launch inside a job, and a job captures every descendant from creation
  onward. Under a CI agent, kill 0 signals the agent and every sibling step.
  The blast radius can be far wider than a POSIX process group.

# Help messages
kill-help-list = Lists signals
kill-help-table = Lists table of signals
kill-help-signal = Sends given signal instead of SIGTERM

# Error messages
kill-error-no-process-id = no process ID specified
  Try --help for more information.
kill-error-invalid-signal = { $signal }: invalid signal
kill-error-parse-argument = failed to parse argument { $argument }: { $error }
kill-error-sending-signal = sending signal to { $pid } failed
kill-error-write = write error: { $error }
kill-error-unsupported-signal = unsupported signal on Windows
kill-error-negative-pid-unsupported = a negative PID (another process's group) is not supported on Windows
