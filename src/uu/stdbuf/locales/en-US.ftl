stdbuf-about = Run COMMAND, with modified buffering operations for its standard streams.

  Mandatory arguments to long options are mandatory for short options too.
stdbuf-usage = stdbuf [OPTION]... COMMAND
stdbuf-after-help = If MODE is 'L' the corresponding stream will be line buffered.
  This option is invalid with standard input.

  If MODE is '0' the corresponding stream will be unbuffered.

  Otherwise, MODE is a number which may be followed by one of the following:

  KB 1000, K 1024, MB 1000*1000, M 1024*1024, and so on for G, T, P, E, Z, Y.
  In this case the corresponding stream will be fully buffered with the buffer size set to MODE bytes.

  NOTE: If COMMAND adjusts the buffering of its standard streams (tee does for e.g.) then that will override corresponding settings changed by stdbuf.
  Also some filters (like dd and cat etc.) don't use streams for I/O, and are thus unaffected by stdbuf settings.

stdbuf-help-input = adjust standard input stream buffering
stdbuf-help-output = adjust standard output stream buffering
stdbuf-help-error = adjust standard error stream buffering
stdbuf-value-mode = MODE

stdbuf-error-line-buffering-stdin-meaningless = line buffering stdin is meaningless
stdbuf-error-invalid-mode = invalid mode {$error}
stdbuf-error-value-too-large = invalid mode '{$value}': Value too large for defined data type
stdbuf-error-command-not-supported = Command not supported for this operating system!
stdbuf-error-external-libstdbuf-not-found = External libstdbuf not found at configured path: {$path}
stdbuf-error-permission-denied = failed to execute process: Permission denied
stdbuf-error-no-such-file = failed to execute process: No such file or directory
stdbuf-error-failed-to-execute = failed to execute process: {$error}
stdbuf-error-killed-by-signal = process killed by signal {$signal}
