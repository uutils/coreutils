# cksum

```
cksum [OPTIONS] [FILE]...
```

Print CRC and size for each file

## After Help

DIGEST determines the digest algorithm and default output format:

- `sysv`:    (equivalent to sum -s)
- `bsd`:     (equivalent to sum -r)
- `crc`:     (equivalent to cksum)
- `md5`:     (equivalent to md5sum)
- `sha1`:    (equivalent to sha1sum)
- `sha224`:  (equivalent to sha224sum)
- `sha256`:  (equivalent to sha256sum)
- `sha384`:  (equivalent to sha384sum)
- `sha512`:  (equivalent to sha512sum)
- `blake2b`: (equivalent to b2sum)
- `sm3`:     (only available through cksum)
