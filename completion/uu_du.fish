complete -c uu_du -s B -l block-size -d 'scale sizes by SIZE before printing them. E.g., \'-BM\' prints sizes in units of 1,048,576 bytes. See SIZE format below.' -r
complete -c uu_du -s d -l max-depth -d 'print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize' -r
complete -c uu_du -s t -l threshold -d 'exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative' -r
complete -c uu_du -l exclude -d 'exclude files that match PATTERN' -r
complete -c uu_du -s X -l exclude-from -d 'exclude files that match any pattern in FILE' -r -F
complete -c uu_du -l files0-from -d 'summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input' -r -F
complete -c uu_du -l time -d 'show time of the last modification of any file in the directory, or any of its subdirectories. If WORD is given, show time as WORD instead of modification time: atime, access, use, ctime, status, birth or creation' -r -f -a "{atime	,ctime	,creation	}"
complete -c uu_du -l time-style -d 'show times using style STYLE: full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like \'date\'' -r
complete -c uu_du -l help -d 'Print help information.'
complete -c uu_du -s a -l all -d 'write counts for all files, not just directories'
complete -c uu_du -l apparent-size -d 'print apparent sizes, rather than disk usage although the apparent size is usually smaller, it may be larger due to holes in (\'sparse\') files, internal fragmentation, indirect blocks, and the like'
complete -c uu_du -s b -l bytes -d 'equivalent to \'--apparent-size --block-size=1\''
complete -c uu_du -s c -l total -d 'produce a grand total'
complete -c uu_du -s h -l human-readable -d 'print sizes in human readable format (e.g., 1K 234M 2G)'
complete -c uu_du -l inodes -d 'list inode usage information instead of block usage like --block-size=1K'
complete -c uu_du -s k -d 'like --block-size=1K'
complete -c uu_du -s l -l count-links -d 'count sizes many times if hard linked'
complete -c uu_du -s L -l dereference -d 'follow all symbolic links'
complete -c uu_du -s D -s H -l dereference-args -d 'follow only symlinks that are listed on the command line'
complete -c uu_du -s P -l no-dereference -d 'don\'t follow any symbolic links (this is the default)'
complete -c uu_du -s m -d 'like --block-size=1M'
complete -c uu_du -s 0 -l null -d 'end each output line with 0 byte rather than newline'
complete -c uu_du -s S -l separate-dirs -d 'do not include size of subdirectories'
complete -c uu_du -s s -l summarize -d 'display only a total for each argument'
complete -c uu_du -l si -d 'like -h, but use powers of 1000 not 1024'
complete -c uu_du -s x -l one-file-system -d 'skip directories on different file systems'
complete -c uu_du -s v -l verbose -d 'verbose mode (option not present in GNU/Coreutils)'
complete -c uu_du -s V -l version -d 'Print version'
