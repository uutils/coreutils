kill-about = Send signal to processes or list information about signals.
kill-usage = kill [OPTIONS]... PID...

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
