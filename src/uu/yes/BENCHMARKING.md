# Benchmarking yes

`yes` is a utility printing the provided string and a newline continuously.
The default value `y` is used to skip the common `y/n` prompts provided by scripts.

## Understanding yes

The most simple way to implement `yes` is the `println!` loop. But it is slow since it
calls many write syscalls. So `yes` should print an extended string to few bytes in the loop for
better throughput.

It is difficult to use `hyperfine` for benchmarking `yes` as it has infinite loop.
But you can see throughput of `yes` by `pv` instead:

```shell
yes | pv >/dev/null
```

### `tee()` zero-copy

The `read()` and `write()` based `yes` implementation is much slower than RAM's bandwidth
since those syscalls are copying content of RAM.

On Linux, `tee()` and `splice()` for non-pipe output is used to avoid copying content of RAM.
