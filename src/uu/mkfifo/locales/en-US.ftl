mkfifo-about = Create a FIFO with the given name.
mkfifo-usage = mkfifo [OPTION]... NAME...

# Help messages
mkfifo-help-mode = file permissions for the fifo
mkfifo-help-selinux = set the SELinux security context to default type
mkfifo-help-context = like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX

# Error messages
mkfifo-error-invalid-mode = invalid mode: { $error }
mkfifo-error-missing-operand = missing operand
mkfifo-error-cannot-create-fifo = cannot create fifo { $path }: File exists
mkfifo-error-cannot-set-permissions = cannot set permissions on { $path }: { $error }
