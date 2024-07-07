complete -c uu_chgrp -l reference -d 'use RFILE\'s group rather than specifying GROUP values' -r -F
complete -c uu_chgrp -l help -d 'Print help information.'
complete -c uu_chgrp -s c -l changes -d 'like verbose but report only when a change is made'
complete -c uu_chgrp -s f -l silent
complete -c uu_chgrp -l quiet -d 'suppress most error messages'
complete -c uu_chgrp -s v -l verbose -d 'output a diagnostic for every file processed'
complete -c uu_chgrp -l dereference
complete -c uu_chgrp -s h -l no-dereference -d 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)'
complete -c uu_chgrp -l preserve-root -d 'fail to operate recursively on \'/\''
complete -c uu_chgrp -l no-preserve-root -d 'do not treat \'/\' specially (the default)'
complete -c uu_chgrp -s R -l recursive -d 'operate on files and directories recursively'
complete -c uu_chgrp -s H -d 'if a command line argument is a symbolic link to a directory, traverse it'
complete -c uu_chgrp -s P -d 'do not traverse any symbolic links (default)'
complete -c uu_chgrp -s L -d 'traverse every symbolic link to a directory encountered'
complete -c uu_chgrp -s V -l version -d 'Print version'
