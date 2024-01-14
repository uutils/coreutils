# nohup

```
nohup COMMAND [ARG]...
nohup OPTION
```

Run COMMAND ignoring hangup signals.

## After Help

If standard input is terminal, it'll be replaced with /dev/null.
If standard output is terminal, it'll be appended to nohup.out instead,
or $HOME/nohup.out, if nohup.out open failed.
If standard error is terminal, it'll be redirected to stdout.
