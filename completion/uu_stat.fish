complete -c uu_stat -s c -l format -d 'use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT' -r
complete -c uu_stat -l printf -d 'like --format, but interpret backslash escapes,
            and do not output a mandatory trailing newline;
            if you want a newline, include 
 in FORMAT' -r
complete -c uu_stat -s L -l dereference -d 'follow links'
complete -c uu_stat -s f -l file-system -d 'display file system status instead of file status'
complete -c uu_stat -s t -l terse -d 'print the information in terse form'
complete -c uu_stat -s h -l help -d 'Print help'
complete -c uu_stat -s V -l version -d 'Print version'
