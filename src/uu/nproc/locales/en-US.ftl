nproc-about = Print the number of cores available to the current process.
  If the OMP_NUM_THREADS or OMP_THREAD_LIMIT environment variables are set, then
  they will determine the minimum and maximum returned value respectively.
nproc-usage = nproc [OPTIONS]...

# Error messages
nproc-error-invalid-number = { $value } is not a valid number: { $error }

# Help text for command-line arguments
nproc-help-all = print the number of cores available to the system
nproc-help-ignore = ignore up to N cores
