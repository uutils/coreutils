complete -c uu_touch -s t -d 'use [[CC]YY]MMDDhhmm[.ss] instead of the current time' -r
complete -c uu_touch -s d -l date -d 'parse argument and use it instead of current time' -r
complete -c uu_touch -s r -l reference -d 'use this file\'s times instead of the current time' -r -F
complete -c uu_touch -l time -d 'change only the specified time: "access", "atime", or "use" are equivalent to -a; "modify" or "mtime" are equivalent to -m' -r -f -a "{atime	,mtime	}"
complete -c uu_touch -l help -d 'Print help information.'
complete -c uu_touch -s a -d 'change only the access time'
complete -c uu_touch -s m -d 'change only the modification time'
complete -c uu_touch -s c -l no-create -d 'do not create any files'
complete -c uu_touch -s h -l no-dereference -d 'affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)'
complete -c uu_touch -s V -l version -d 'Print version'
