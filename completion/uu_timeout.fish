complete -c uu_timeout -s k -l kill-after -d 'also send a KILL signal if COMMAND is still running this long after the initial signal was sent' -r
complete -c uu_timeout -s s -l signal -d 'specify the signal to be sent on timeout; SIGNAL may be a name like \'HUP\' or a number; see \'kill -l\' for a list of signals' -r
complete -c uu_timeout -l foreground -d 'when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out'
complete -c uu_timeout -l preserve-status -d 'exit with the same status as COMMAND, even when the command times out'
complete -c uu_timeout -s v -l verbose -d 'diagnose to stderr any signal sent upon timeout'
complete -c uu_timeout -s h -l help -d 'Print help'
complete -c uu_timeout -s V -l version -d 'Print version'
