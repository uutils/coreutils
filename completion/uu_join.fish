complete -c uu_join -s a -d 'also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2' -r -f -a "{1	,2	}"
complete -c uu_join -s v -d 'like -a FILENUM, but suppress joined output lines' -r -f -a "{1	,2	}"
complete -c uu_join -s e -d 'replace missing input fields with EMPTY' -r
complete -c uu_join -s j -d 'equivalent to \'-1 FIELD -2 FIELD\'' -r
complete -c uu_join -s o -d 'obey FORMAT while constructing output line' -r
complete -c uu_join -s t -d 'use CHAR as input and output field separator' -r
complete -c uu_join -s 1 -d 'join on this FIELD of file 1' -r
complete -c uu_join -s 2 -d 'join on this FIELD of file 2' -r
complete -c uu_join -s i -l ignore-case -d 'ignore differences in case when comparing fields'
complete -c uu_join -l check-order -d 'check that the input is correctly sorted, even if all input lines are pairable'
complete -c uu_join -l nocheck-order -d 'do not check that the input is correctly sorted'
complete -c uu_join -l header -d 'treat the first line in each file as field headers, print them without trying to pair them'
complete -c uu_join -s z -l zero-terminated -d 'line delimiter is NUL, not newline'
complete -c uu_join -s h -l help -d 'Print help'
complete -c uu_join -s V -l version -d 'Print version'
