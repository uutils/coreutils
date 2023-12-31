# basenc

```
basenc [OPTION]... [FILE]
```

Encode/decode data and print to standard output
With no FILE, or when FILE is -, read standard input.

When decoding, the input may contain newlines in addition to the bytes of
the formal alphabet. Use --ignore-garbage to attempt to recover
from any other non-alphabet bytes in the encoded stream.
