# base32

```
base32 [OPTION]... [FILE]
```

encode/decode data and print to standard output
With no FILE, or when FILE is -, read standard input.

The data are encoded as described for the base32 alphabet in RFC 4648.
When decoding, the input may contain newlines in addition
to the bytes of the formal base32 alphabet. Use --ignore-garbage
to attempt to recover from any other non-alphabet bytes in the
encoded stream.
