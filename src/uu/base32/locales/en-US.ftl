# This file contains base32, base64 and basenc strings
# This is because we have some common strings for all these tools
# and it is easier to have a single file than one file for program
# and loading several bundles at the same time.

base32-about = encode/decode data and print to standard output
  With no FILE, or when FILE is -, read standard input.

  The data are encoded as described for the base32 alphabet in RFC 4648.
  When decoding, the input may contain newlines in addition
  to the bytes of the formal base32 alphabet. Use --ignore-garbage
  to attempt to recover from any other non-alphabet bytes in the
  encoded stream.
base32-usage = base32 [OPTION]... [FILE]

base64-about = encode/decode data and print to standard output
  With no FILE, or when FILE is -, read standard input.

  The data are encoded as described for the base64 alphabet in RFC 3548.
  When decoding, the input may contain newlines in addition
  to the bytes of the formal base64 alphabet. Use --ignore-garbage
  to attempt to recover from any other non-alphabet bytes in the
  encoded stream.
base64-usage = base64 [OPTION]... [FILE]

basenc-about = Encode/decode data and print to standard output
  With no FILE, or when FILE is -, read standard input.

  When decoding, the input may contain newlines in addition to the bytes of
  the formal alphabet. Use --ignore-garbage to attempt to recover
  from any other non-alphabet bytes in the encoded stream.
basenc-usage = basenc [OPTION]... [FILE]

# Help messages for encoding formats
basenc-help-base64 = same as 'base64' program
basenc-help-base64url = file- and url-safe base64
basenc-help-base32 = same as 'base32' program
basenc-help-base32hex = extended hex alphabet base32
basenc-help-base16 = hex encoding
basenc-help-base2lsbf = bit string with least significant bit (lsb) first
basenc-help-base2msbf = bit string with most significant bit (msb) first
basenc-help-z85 = ascii85-like encoding;
  when encoding, input length must be a multiple of 4;
  when decoding, input length must be a multiple of 5
basenc-help-base58 = visually unambiguous base58 encoding

# Error messages
basenc-error-missing-encoding-type = missing encoding type

# Shared base_common error messages (used by base32, base64, basenc)
base-common-extra-operand = extra operand {$operand}
base-common-no-such-file = {$file}: No such file or directory
base-common-invalid-wrap-size = invalid wrap size: {$size}
base-common-read-error = read error: {$error}

# Shared base_common help messages
base-common-help-decode = decode data
base-common-help-ignore-garbage = when decoding, ignore non-alphabetic characters
base-common-help-wrap = wrap encoded lines after COLS character (default {$default}, 0 to disable wrapping)
