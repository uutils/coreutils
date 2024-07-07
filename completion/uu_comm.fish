complete -c uu_comm -l output-delimiter -d 'separate columns with STR' -r
complete -c uu_comm -s 1 -d 'suppress column 1 (lines unique to FILE1)'
complete -c uu_comm -s 2 -d 'suppress column 2 (lines unique to FILE2)'
complete -c uu_comm -s 3 -d 'suppress column 3 (lines that appear in both files)'
complete -c uu_comm -s z -l zero-terminated -d 'line delimiter is NUL, not newline'
complete -c uu_comm -l total -d 'output a summary'
complete -c uu_comm -s h -l help -d 'Print help'
complete -c uu_comm -s V -l version -d 'Print version'
