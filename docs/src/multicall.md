#  Multi-call binary
uutils includes a multi-call binary from which the utils can be invoked. This
reduces the binary size of the binary and can be useful for portability.

The first argument of the multi-call binary is the util to run, after which
the regular arguments to the util can be passed.

```shell
coreutils [util] [util options]
```

The `--help` flag will print a list of available utils.

## Example
```
coreutils ls -l
```