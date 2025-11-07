timeout-about = Start COMMAND, and kill it if still running after DURATION.
timeout-usage = timeout [OPTION] DURATION COMMAND...

# Help messages
timeout-help-foreground = when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out
timeout-help-kill-after = also send a KILL signal if COMMAND is still running this long after the initial signal was sent
timeout-help-preserve-status = exit with the same status as COMMAND, even when the command times out
timeout-help-signal = specify the signal to be sent on timeout; SIGNAL may be a name like 'HUP' or a number; see 'kill -l' for a list of signals
timeout-help-verbose = diagnose to stderr any signal sent upon timeout
timeout-help-duration = a floating point number with an optional suffix: 's' for seconds (the default), 'm' for minutes, 'h' for hours or 'd' for days ; a duration of 0 disables the associated timeout
timeout-help-command = a command to execute with optional arguments
timeout-after-help = Upon timeout, send the TERM signal to COMMAND, if no other SIGNAL specified. The TERM signal kills any process that does not block or catch that signal. It may be necessary to use the KILL signal, since this signal can't be caught.

  Exit status:
    124  if COMMAND times out, and --preserve-status is not specified
    125  if the timeout command itself fails
    126  if COMMAND is found but cannot be invoked
    127  if COMMAND cannot be found
    137  if COMMAND (or timeout itself) is sent the KILL (9) signal (128+9)
    -    the exit status of COMMAND otherwise

# Error messages
timeout-error-invalid-signal = { $signal }: invalid signal
timeout-error-failed-to-execute-process = failed to execute process: { $error }

# Verbose messages
timeout-verbose-sending-signal = sending signal { $signal } to command { $command }
