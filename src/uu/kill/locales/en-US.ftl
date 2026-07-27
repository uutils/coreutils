kill-about = Send signal to processes or list information about signals.
kill-usage = kill [OPTIONS]... PID...
kill-after-help-windows = Windows notes:
  Signalled processes are force-terminated (Windows has no signal delivery);
  their exit status is 128 plus the signal number. Process groups (PID <= 0)
  and STOP are not supported. Permissions come from your current token: kill
  never enables SeDebugPrivilege, so an elevated kill may report 'Permission
  denied' where 'taskkill /F' succeeds.

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
