base32-about = encode/decode data and print to standard output
  With no FILE, or when FILE is -, read standard input.

  The data are encoded as described for the base32 alphabet in RFC 4648.
  When decoding, the input may contain newlines in addition
  to the bytes of the formal base32 alphabet. Use --ignore-garbage
  to attempt to recover from any other non-alphabet bytes in the
  encoded stream.
base32-usage = base32 [OPTION]... [FILE]

# Error messages
base32-extra-operand = extra operand {$operand}
base32-no-such-file = {$file}: No such file or directory
base32-invalid-wrap-size = invalid wrap size: {$size}
base32-read-error = read error: {$error}

# Help messages
base32-help-decode = decode data
base32-help-ignore-garbage = when decoding, ignore non-alphabetic characters
base32-help-wrap = wrap encoded lines after COLS character (default {$default}, 0 to disable wrapping)
