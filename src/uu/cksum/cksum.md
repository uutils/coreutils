# cksum

```
cksum [OPTIONS] [FILE]...
```

Print CRC and size for each file

## After Help

DIGEST determines the digest algorithm and default output format:

-a=sysv:    (equivalent to sum -s)
-a=bsd:     (equivalent to sum -r)
-a=crc:     (equivalent to cksum)
-a=md5:     (equivalent to md5sum)
-a=sha1:    (equivalent to sha1sum)
-a=sha224:  (equivalent to sha224sum)
-a=sha256:  (equivalent to sha256sum)
-a=sha384:  (equivalent to sha384sum)
-a=sha512:  (equivalent to sha512sum)
-a=blake2b: (equivalent to b2sum)
-a=sm3:     (only available through cksum)
