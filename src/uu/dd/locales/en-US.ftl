dd-about = Copy, and optionally convert, a file system resource
dd-usage = dd [OPERAND]...
  dd OPTION
dd-after-help = ### Operands

  - bs=BYTES : read and write up to BYTES bytes at a time (default: 512);
     overwrites ibs and obs.
  - cbs=BYTES : the 'conversion block size' in bytes. Applies to the
     conv=block, and conv=unblock operations.
  - conv=CONVS : a comma-separated list of conversion options or (for legacy
     reasons) file flags.
  - count=N : stop reading input after N ibs-sized read operations rather
     than proceeding until EOF. See iflag=count_bytes if stopping after N bytes
     is preferred
  - ibs=N : the size of buffer used for reads (default: 512)
  - if=FILE : the file used for input. When not specified, stdin is used instead
  - iflag=FLAGS : a comma-separated list of input flags which specify how the
     input source is treated. FLAGS may be any of the input-flags or general-flags
     specified below.
  - skip=N (or iseek=N) : skip N ibs-sized records into input before beginning
     copy/convert operations. See iflag=seek_bytes if seeking N bytes is preferred.
  - obs=N : the size of buffer used for writes (default: 512)
  - of=FILE : the file used for output. When not specified, stdout is used
     instead
  - oflag=FLAGS : comma separated list of output flags which specify how the
     output source is treated. FLAGS may be any of the output flags or general
     flags specified below
  - seek=N (or oseek=N) : seeks N obs-sized records into output before
     beginning copy/convert operations. See oflag=seek_bytes if seeking N bytes is
     preferred
  - status=LEVEL : controls whether volume and performance stats are written to
     stderr.

    When unspecified, dd will print stats upon completion. An example is below.

    ```plain
      6+0 records in
      16+0 records out
      8192 bytes (8.2 kB, 8.0 KiB) copied, 0.00057009 s,
      14.4 MB/s

    The first two lines are the 'volume' stats and the final line is the
    'performance' stats.
    The volume stats indicate the number of complete and partial ibs-sized reads,
    or obs-sized writes that took place during the copy. The format of the volume
    stats is <complete>+<partial>. If records have been truncated (see
    conv=block), the volume stats will contain the number of truncated records.

    Possible LEVEL values are:
    - progress : Print periodic performance stats as the copy proceeds.
    - noxfer : Print final volume stats, but not performance stats.
    - none : Do not print any stats.

    Printing performance stats is also triggered by the INFO signal (where supported),
    or the USR1 signal. Setting the POSIXLY_CORRECT environment variable to any value
    (including an empty value) will cause the USR1 signal to be ignored.

  ### Conversion Options

  - ascii : convert from EBCDIC to ASCII. This is the inverse of the ebcdic
    option. Implies conv=unblock.
  - ebcdic : convert from ASCII to EBCDIC. This is the inverse of the ascii
    option. Implies conv=block.
  - ibm : convert from ASCII to EBCDIC, applying the conventions for [, ]
    and ~ specified in POSIX. Implies conv=block.

  - ucase : convert from lower-case to upper-case.
  - lcase : converts from upper-case to lower-case.

  - block : for each newline less than the size indicated by cbs=BYTES, remove
    the newline and pad with spaces up to cbs. Lines longer than cbs are truncated.
  - unblock : for each block of input of the size indicated by cbs=BYTES, remove
    right-trailing spaces and replace with a newline character.

  - sparse : attempts to seek the output when an obs-sized block consists of
    only zeros.
  - swab : swaps each adjacent pair of bytes. If an odd number of bytes is
    present, the final byte is omitted.
  - sync : pad each ibs-sided block with zeros. If block or unblock is
    specified, pad with spaces instead.
  - excl : the output file must be created. Fail if the output file is already
    present.
  - nocreat : the output file will not be created. Fail if the output file in
    not already present.
  - notrunc : the output file will not be truncated. If this option is not
    present, output will be truncated when opened.
  - noerror : all read errors will be ignored. If this option is not present,
    dd will only ignore Error::Interrupted.
  - fdatasync : data will be written before finishing.
  - fsync : data and metadata will be written before finishing.

  ### Input flags

  - count_bytes : a value to count=N will be interpreted as bytes.
  - skip_bytes : a value to skip=N will be interpreted as bytes.
  - fullblock : wait for ibs bytes from each read. zero-length reads are still
    considered EOF.

  ### Output flags

  - append : open file in append mode. Consider setting conv=notrunc as well.
  - seek_bytes : a value to seek=N will be interpreted as bytes.

  ### General Flags

  - direct : use direct I/O for data.
  - directory : fail unless the given input (if used as an iflag) or
    output (if used as an oflag) is a directory.
  - dsync : use synchronized I/O for data.
  - sync : use synchronized I/O for data and metadata.
  - nonblock : use non-blocking I/O.
  - noatime : do not update access time.
  - nocache : request that OS drop cache.
  - noctty : do not assign a controlling tty.
  - nofollow : do not follow system links.

# Common strings
dd-standard-input = 'standard input'
dd-standard-output = 'standard output'

# Error messages
dd-error-failed-to-open = failed to open { $path }
dd-error-write-error = write error
dd-error-failed-to-seek = failed to seek in output file
dd-error-io-error = IO error
dd-error-cannot-skip-offset = '{ $file }': cannot skip to specified offset
dd-error-cannot-skip-invalid = '{ $file }': cannot skip: Invalid argument
dd-error-cannot-seek-invalid = '{ $output }': cannot seek: Invalid argument
dd-error-not-directory = setting flags for '{ $file }': Not a directory
dd-error-failed-discard-cache = failed to discard cache for: { $file }

# Parse errors
dd-error-unrecognized-operand = Unrecognized operand '{ $operand }'
dd-error-multiple-format-table = Only one of conv=ascii conv=ebcdic or conv=ibm may be specified
dd-error-multiple-case = Only one of conv=lcase or conv=ucase may be specified
dd-error-multiple-block = Only one of conv=block or conv=unblock may be specified
dd-error-multiple-excl = Only one ov conv=excl or conv=nocreat may be specified
dd-error-invalid-flag = invalid input flag: ‘{ $flag }’
  Try '{ $cmd } --help' for more information.
dd-error-conv-flag-no-match = Unrecognized conv=CONV -> { $flag }
dd-error-multiplier-parse-failure = invalid number: '{ $input }'
dd-error-multiplier-overflow = Multiplier string would overflow on current system -> { $input }
dd-error-block-without-cbs = conv=block or conv=unblock specified without cbs=N
dd-error-status-not-recognized = status=LEVEL not recognized -> { $level }
dd-error-unimplemented = feature not implemented on this system -> { $feature }
dd-error-bs-out-of-range = { $param }=N cannot fit into memory
dd-error-invalid-number = invalid number: ‘{ $input }’

# Progress messages
dd-progress-records-in = { $complete }+{ $partial } records in
dd-progress-records-out = { $complete }+{ $partial } records out
dd-progress-truncated-record = { $count ->
    [one] { $count } truncated record
   *[other] { $count } truncated records
}
dd-progress-byte-copied = { $bytes } byte copied, { $duration } s, { $rate }/s
dd-progress-bytes-copied = { $bytes } bytes copied, { $duration } s, { $rate }/s
dd-progress-bytes-copied-si = { $bytes } bytes ({ $si }) copied, { $duration } s, { $rate }/s
dd-progress-bytes-copied-si-iec = { $bytes } bytes ({ $si }, { $iec }) copied, { $duration } s, { $rate }/s

# Warnings
dd-warning-zero-multiplier = { $zero } is a zero multiplier; use { $alternative } if that is intended
dd-warning-signal-handler = Internal dd Warning: Unable to register signal handler
