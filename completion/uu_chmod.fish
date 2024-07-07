complete -c uu_chmod -l reference -d 'use RFILE\'s mode instead of MODE values' -r -F
complete -c uu_chmod -s c -l changes -d 'like verbose but report only when a change is made'
complete -c uu_chmod -s f -l quiet -l silent -d 'suppress most error messages'
complete -c uu_chmod -s v -l verbose -d 'output a diagnostic for every file processed'
complete -c uu_chmod -l no-preserve-root -d 'do not treat \'/\' specially (the default)'
complete -c uu_chmod -l preserve-root -d 'fail to operate recursively on \'/\''
complete -c uu_chmod -s R -l recursive -d 'change files and directories recursively'
complete -c uu_chmod -s h -l help -d 'Print help'
complete -c uu_chmod -s V -l version -d 'Print version'
