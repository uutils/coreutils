tail-about = Print the last 10 lines of each FILE to standard output.
  With more than one FILE, precede each with a header giving the file name.
  With no FILE, or when FILE is -, read standard input.
  Mandatory arguments to long flags are mandatory for short flags too.
tail-usage = tail [FLAG]... [FILE]...

# Help messages
tail-help-bytes = Number of bytes to print
tail-help-follow = Print the file as it grows
tail-help-lines = Number of lines to print
tail-help-pid = With -f, terminate after process ID, PID dies
tail-help-quiet = Never output headers giving file names
tail-help-sleep-interval = Number of seconds to sleep between polling the file when running with -f
tail-help-max-unchanged-stats = Reopen a FILE which has not changed size after N (default 5) iterations to see if it has been unlinked or renamed (this is the usual case of rotated log files); This option is meaningful only when polling (i.e., with --use-polling) and when --follow=name
tail-help-verbose = Always output headers giving file names
tail-help-zero-terminated = Line delimiter is NUL, not newline
tail-help-retry = Keep trying to open a file if it is inaccessible
tail-help-follow-retry = Same as --follow=name --retry
tail-help-polling-linux = Disable 'inotify' support and use polling instead
tail-help-polling-unix = Disable 'kqueue' support and use polling instead
tail-help-polling-windows = Disable 'ReadDirectoryChanges' support and use polling instead

# Error messages
tail-error-cannot-follow-stdin-by-name = cannot follow { $stdin } by name
tail-error-cannot-open-no-such-file = cannot open '{ $file }' for reading: { $error }
tail-error-reading-file = error reading '{ $file }': { $error }
tail-error-cannot-follow-file-type = { $file }: cannot follow end of this type of file{ $msg }
tail-error-cannot-open-for-reading = cannot open '{ $file }' for reading
tail-error-cannot-fstat = cannot fstat { $file }: { $error }
tail-error-invalid-number-of-bytes = invalid number of bytes: { $arg }
tail-error-invalid-number-of-lines = invalid number of lines: { $arg }
tail-error-invalid-number-of-seconds = invalid number of seconds: '{ $source }'
tail-error-invalid-max-unchanged-stats = invalid maximum number of unchanged stats between opens: { $value }
tail-error-invalid-pid = invalid PID: { $pid }
tail-error-invalid-pid-with-error = invalid PID: { $pid }: { $error }
tail-error-invalid-number-out-of-range = invalid number: { $arg }: Numerical result out of range
tail-error-invalid-number-overflow = invalid number: { $arg }
tail-error-option-used-in-invalid-context = option used in invalid context -- { $option }
tail-error-bad-argument-encoding = bad argument encoding: { $arg }
tail-error-cannot-watch-parent-directory = cannot watch parent directory of { $path }
tail-error-backend-cannot-be-used-too-many-files = { $backend } cannot be used, reverting to polling: Too many open files
tail-error-backend-resources-exhausted = { $backend } resources exhausted
tail-error-notify-error = NotifyError: { $error }
tail-error-recv-timeout-error = RecvTimeoutError: { $error }

# Warning messages
tail-warning-retry-ignored = --retry ignored; --retry is useful only when following
tail-warning-retry-only-effective = --retry only effective for the initial open
tail-warning-pid-ignored = PID ignored; --pid=PID is useful only when following
tail-warning-pid-not-supported = --pid=PID is not supported on this system
tail-warning-following-stdin-ineffective = following standard input indefinitely is ineffective

# Status messages
tail-status-has-become-accessible = { $file } has become accessible
tail-status-has-appeared-following-new-file = { $file } has appeared;  following new file
tail-status-has-been-replaced-following-new-file = { $file } has been replaced;  following new file
tail-status-file-truncated = { $file }: file truncated
tail-status-replaced-with-untailable-file = { $file } has been replaced with an untailable file
tail-status-replaced-with-untailable-file-giving-up = { $file } has been replaced with an untailable file; giving up on this name
tail-status-file-became-inaccessible = { $file } { $become_inaccessible }: { $no_such_file }
tail-status-directory-containing-watched-file-removed = directory containing watched file was removed
tail-status-backend-cannot-be-used-reverting-to-polling = { $backend } cannot be used, reverting to polling
tail-status-file-no-such-file = { $file }: { $no_such_file }

# Text constants
tail-bad-fd = Bad file descriptor
tail-no-such-file-or-directory = No such file or directory
tail-is-a-directory = Is a directory
tail-giving-up-on-this-name = ; giving up on this name
tail-stdin-header = standard input
tail-no-files-remaining = no files remaining
tail-become-inaccessible = has become inaccessible
