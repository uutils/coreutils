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

# Error messages
basenc-error-missing-encoding-type = missing encoding type
