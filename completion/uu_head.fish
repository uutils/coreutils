complete -c uu_head -s c -l bytes -d 'print the first NUM bytes of each file;
with the leading \'-\', print all but the last
NUM bytes of each file' -r
complete -c uu_head -s n -l lines -d 'print the first NUM lines instead of the first 10;
with the leading \'-\', print all but the last
NUM lines of each file' -r
complete -c uu_head -s q -l quiet -l silent -d 'never print headers giving file names'
complete -c uu_head -s v -l verbose -d 'always print headers giving file names'
complete -c uu_head -l presume-input-pipe
complete -c uu_head -s z -l zero-terminated -d 'line delimiter is NUL, not newline'
complete -c uu_head -s h -l help -d 'Print help'
complete -c uu_head -s V -l version -d 'Print version'
