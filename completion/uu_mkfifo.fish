complete -c uu_mkfifo -s m -l mode -d 'file permissions for the fifo' -r
complete -c uu_mkfifo -l context -d 'like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX' -r
complete -c uu_mkfifo -s Z -d 'set the SELinux security context to default type'
complete -c uu_mkfifo -s h -l help -d 'Print help'
complete -c uu_mkfifo -s V -l version -d 'Print version'
