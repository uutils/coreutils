mknod-about = Create the special file NAME of the given TYPE.
mknod-usage = mknod [OPTION]... NAME TYPE [MAJOR MINOR]
mknod-after-help = Mandatory arguments to long options are mandatory for short options too.
  -m, --mode=MODE set file permission bits to MODE, not a=rw - umask

  Both MAJOR and MINOR must be specified when TYPE is b, c, or u, and they
  must be omitted when TYPE is p. If MAJOR or MINOR begins with 0x or 0X,
  it is interpreted as hexadecimal; otherwise, if it begins with 0, as octal;
  otherwise, as decimal. TYPE may be:

  - b create a block (buffered) special file
  - c, u create a character (unbuffered) special file
  - p create a FIFO

  NOTE: your shell may have its own version of mknod, which usually supersedes
  the version described here. Please refer to your shell's documentation
  for details about the options it supports.

# Help messages
mknod-help-mode = set file permission bits to MODE, not a=rw - umask
mknod-help-name = name of the new file
mknod-help-type = type of the new file (b, c, u or p)
mknod-help-major = major file type
mknod-help-minor = minor file type
mknod-help-selinux = set SELinux security context of each created directory to the default type
mknod-help-context = like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX

# Error messages
mknod-error-fifo-no-major-minor = Fifos do not have major and minor device numbers.
mknod-error-special-require-major-minor = Special files require major and minor device numbers.
mknod-error-invalid-mode = invalid mode ({ $error })
mknod-error-mode-permission-bits-only = mode must specify only file permission bits
mknod-error-missing-device-type = missing device type
mknod-error-invalid-device-type = invalid device type { $type }
