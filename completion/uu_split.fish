complete -c uu_split -s b -l bytes -d 'put SIZE bytes per output file' -r
complete -c uu_split -s C -l line-bytes -d 'put at most SIZE bytes of lines per output file' -r
complete -c uu_split -s l -l lines -d 'put NUMBER lines/records per output file' -r
complete -c uu_split -s n -l number -d 'generate CHUNKS output files; see explanation below' -r
complete -c uu_split -l additional-suffix -d 'additional SUFFIX to append to output file names' -r
complete -c uu_split -l filter -d 'write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)' -r -f -a "(__fish_complete_command)"
complete -c uu_split -l numeric-suffixes -d 'same as -d, but allow setting the start value' -r
complete -c uu_split -l hex-suffixes -d 'same as -x, but allow setting the start value' -r
complete -c uu_split -s a -l suffix-length -d 'generate suffixes of length N (default 2)' -r
complete -c uu_split -s t -l separator -d 'use SEP instead of newline as the record separator; \'\\0\' (zero) specifies the NUL character' -r
complete -c uu_split -l io-blksize -r
complete -c uu_split -s e -l elide-empty-files -d 'do not generate empty output files with \'-n\''
complete -c uu_split -s d -d 'use numeric suffixes starting at 0, not alphabetic'
complete -c uu_split -s x -d 'use hex suffixes starting at 0, not alphabetic'
complete -c uu_split -l verbose -d 'print a diagnostic just before each output file is opened'
complete -c uu_split -s h -l help -d 'Print help'
complete -c uu_split -s V -l version -d 'Print version'
