complete -c uu_chown -l from -d 'change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute' -r
complete -c uu_chown -l reference -d 'use RFILE\'s owner and group rather than specifying OWNER:GROUP values' -r -F
complete -c uu_chown -l help -d 'Print help information.'
complete -c uu_chown -s c -l changes -d 'like verbose but report only when a change is made'
complete -c uu_chown -l dereference -d 'affect the referent of each symbolic link (this is the default), rather than the symbolic link itself'
complete -c uu_chown -s h -l no-dereference -d 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)'
complete -c uu_chown -l preserve-root -d 'fail to operate recursively on \'/\''
complete -c uu_chown -l no-preserve-root -d 'do not treat \'/\' specially (the default)'
complete -c uu_chown -l quiet -d 'suppress most error messages'
complete -c uu_chown -s R -l recursive -d 'operate on files and directories recursively'
complete -c uu_chown -s f -l silent
complete -c uu_chown -s H -d 'if a command line argument is a symbolic link to a directory, traverse it'
complete -c uu_chown -s L -d 'traverse every symbolic link to a directory encountered'
complete -c uu_chown -s P -d 'do not traverse any symbolic links (default)'
complete -c uu_chown -s v -l verbose -d 'output a diagnostic for every file processed'
complete -c uu_chown -s V -l version -d 'Print version'
