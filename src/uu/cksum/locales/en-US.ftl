cksum-about = Print CRC and size for each file
cksum-usage = cksum [OPTIONS] [FILE]...
cksum-after-help = DIGEST determines the digest algorithm and default output format:

  - sysv: (equivalent to sum -s)
  - bsd: (equivalent to sum -r)
  - crc: (equivalent to cksum)
  - crc32b: (only available through cksum)
  - md5: (equivalent to md5sum)
  - sha1: (equivalent to sha1sum)
  - sha2: (equivalent to sha{"{224,256,384,512}"}sum)
  - sha3: (only available through cksum)
  - blake2b: (equivalent to b2sum)
  - sm3: (only available through cksum)
