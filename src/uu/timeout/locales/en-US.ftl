timeout-about = Start COMMAND, and kill it if still running after DURATION.
timeout-usage = timeout [OPTION] DURATION COMMAND...

# Help messages
timeout-help-foreground = when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out
timeout-help-kill-after = also send a KILL signal if COMMAND is still running this long after the initial signal was sent
timeout-help-preserve-status = exit with the same status as COMMAND, even when the command times out
timeout-help-signal = specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals
timeout-help-verbose = diagnose to stderr any signal sent upon timeout

# Error messages
timeout-error-invalid-signal = { $signal }: invalid signal
timeout-error-failed-to-execute-process = failed to execute process: { $error }

# Verbose messages
timeout-verbose-sending-signal = sending signal { $signal } to command { $command }
