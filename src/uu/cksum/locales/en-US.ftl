cksum-about = Print checksum and size for each file
cksum-usage = cksum (multicall-binary for *sum) [OPTIONS] [FILE]...
cksum-after-help = DIGEST determines the digest algorithm and default output format:

  - sysv: (equivalent to sum -s)
  - bsd: (equivalent to sum -r)
  - crc: (default if binary name is cksum)
  - crc32b: (only available through cksum)
  - md5: (binary name : md5sum)
  - sha1: (binary name : sha1sum)
  - sha2: (binary name : sha{"{224,256,384,512}"}sum)
  - sha3: (only available through cksum)
  - blake2b: (binary name : b2sum)
  - sm3: (only available through cksum)

# Help messages
cksum-help-algorithm = select the digest type to use. See DIGEST below
cksum-help-untagged = create a reversed style checksum, without digest type
cksum-help-tag = create a BSD style checksum (default if binary name is cksum)
cksum-help-length = digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8
cksum-help-raw = emit a raw binary digest, not hexadecimal
cksum-help-strict = exit non-zero for improperly formatted checksum lines
cksum-help-check = read hashsums from the FILEs and check them
cksum-help-base64 = emit a base64 digest, not hexadecimal
cksum-help-warn = warn about improperly formatted checksum lines
cksum-help-status = don't output anything, status code shows success
cksum-help-quiet = don't print OK for each successfully verified file
cksum-help-ignore-missing = don't fail or report status for missing files
cksum-help-zero = end each output line with NUL, not newline, and disable file name escaping
cksum-help-debug = print CPU hardware capability detection info used by cksum
