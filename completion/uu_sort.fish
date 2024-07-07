complete -c uu_sort -l sort -r -f -a "{general-numeric	,human-numeric	,month	,numeric	,version	,random	}"
complete -c uu_sort -s c -l check -d 'check for sorted input; do not sort' -r -f -a "{silent	,quiet	,diagnose-first	}"
complete -c uu_sort -s o -l output -d 'write output to FILENAME instead of stdout' -r -F
complete -c uu_sort -s k -l key -d 'sort by a key' -r
complete -c uu_sort -s t -l field-separator -d 'custom separator for -k' -r
complete -c uu_sort -l parallel -d 'change the number of threads running concurrently to NUM_THREADS' -r
complete -c uu_sort -s S -l buffer-size -d 'sets the maximum SIZE of each segment in number of sorted items' -r
complete -c uu_sort -s T -l temporary-directory -d 'use DIR for temporaries, not $TMPDIR or /tmp' -r -f -a "(__fish_complete_directories)"
complete -c uu_sort -l compress-program -d 'compress temporary files with PROG, decompress with PROG -d; PROG has to take input from stdin and output to stdout' -r -f -a "(__fish_complete_command)"
complete -c uu_sort -l batch-size -d 'Merge at most N_MERGE inputs at once.' -r
complete -c uu_sort -l files0-from -d 'read input from the files specified by NUL-terminated NUL_FILES' -r -F
complete -c uu_sort -l help -d 'Print help information.'
complete -c uu_sort -l version -d 'Print version information.'
complete -c uu_sort -s h -l human-numeric-sort -d 'compare according to human readable sizes, eg 1M > 100k'
complete -c uu_sort -s M -l month-sort -d 'compare according to month name abbreviation'
complete -c uu_sort -s n -l numeric-sort -d 'compare according to string numerical value'
complete -c uu_sort -s g -l general-numeric-sort -d 'compare according to string general numerical value'
complete -c uu_sort -s V -l version-sort -d 'Sort by SemVer version number, eg 1.12.2 > 1.1.2'
complete -c uu_sort -s R -l random-sort -d 'shuffle in random order'
complete -c uu_sort -s d -l dictionary-order -d 'consider only blanks and alphanumeric characters'
complete -c uu_sort -s m -l merge -d 'merge already sorted files; do not sort'
complete -c uu_sort -s C -l check-silent -d 'exit successfully if the given file is already sorted, and exit with status 1 otherwise.'
complete -c uu_sort -s f -l ignore-case -d 'fold lower case to upper case characters'
complete -c uu_sort -s i -l ignore-nonprinting -d 'ignore nonprinting characters'
complete -c uu_sort -s b -l ignore-leading-blanks -d 'ignore leading blanks when finding sort keys in each line'
complete -c uu_sort -s r -l reverse -d 'reverse the output'
complete -c uu_sort -s s -l stable -d 'stabilize sort by disabling last-resort comparison'
complete -c uu_sort -s u -l unique -d 'output only the first of an equal run'
complete -c uu_sort -s z -l zero-terminated -d 'line delimiter is NUL, not newline'
complete -c uu_sort -l debug -d 'underline the parts of the line that are actually used for sorting'
