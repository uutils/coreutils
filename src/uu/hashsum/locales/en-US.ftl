hashsum-about = Compute and check message digests.
hashsum-usage = hashsum --<digest> [OPTIONS]... [FILE]...

# Utility-specific usage template
hashsum-usage-specific = {$utility_name} [OPTION]... [FILE]...

# Help messages
hashsum-help-binary-windows = read or check in binary mode (default)
hashsum-help-binary-other = read in binary mode
hashsum-help-text-windows = read or check in text mode
hashsum-help-text-other = read in text mode (default)
hashsum-help-check = read hashsums from the FILEs and check them
hashsum-help-tag = create a BSD-style checksum
hashsum-help-quiet = don't print OK for each successfully verified file
hashsum-help-status = don't output anything, status code shows success
hashsum-help-strict = exit non-zero for improperly formatted checksum lines
hashsum-help-ignore-missing = don't fail or report status for missing files
hashsum-help-warn = warn about improperly formatted checksum lines
hashsum-help-zero = end each output line with NUL, not newline
hashsum-help-length = digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8

# Algorithm help messages
hashsum-help-md5 = work with MD5
hashsum-help-sha1 = work with SHA1
hashsum-help-sha224 = work with SHA224
hashsum-help-sha256 = work with SHA256
hashsum-help-sha384 = work with SHA384
hashsum-help-sha512 = work with SHA512
hashsum-help-b2sum = work with BLAKE2

# Error messages
hashsum-error-failed-to-read-input = failed to read input
