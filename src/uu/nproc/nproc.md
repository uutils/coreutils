# nproc

```
nproc [OPTIONS]...
```

Print the number of cores available to the current process.
If the `OMP_NUM_THREADS` or `OMP_THREAD_LIMIT` environment variables are set, then
they will determine the minimum and maximum returned value respectively.
