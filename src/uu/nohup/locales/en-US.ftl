nohup-about = Run COMMAND ignoring hangup signals.
nohup-usage = nohup COMMAND [ARG]...
  nohup OPTION
nohup-after-help = If standard input is terminal, it'll be replaced with /dev/null.
  If standard output is terminal, it'll be appended to nohup.out instead,
  or $HOME/nohup.out, if nohup.out open failed.
  If standard error is terminal, it'll be redirected to stdout.

# Error messages
nohup-error-cannot-detach = Cannot detach from console
nohup-error-cannot-replace = Cannot replace { $name }: { $err }
nohup-error-open-failed = failed to open { $path }: { $err }
nohup-error-open-failed-both = failed to open { $first_path }: { $first_err }\nfailed to open { $second_path }: { $second_err }

# Status messages
nohup-ignoring-input-appending-output = ignoring input and appending output to { $path }
