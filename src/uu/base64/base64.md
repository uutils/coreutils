# base64

```
base64 [OPTION]... [FILE]
```

encode/decode data and print to standard output
With no FILE, or when FILE is -, read standard input.

The data are encoded as described for the base64 alphabet in RFC 3548.
When decoding, the input may contain newlines in addition
to the bytes of the formal base64 alphabet. Use --ignore-garbage
to attempt to recover from any other non-alphabet bytes in the
encoded stream.
