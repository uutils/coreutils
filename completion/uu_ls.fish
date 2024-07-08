complete -c uu_ls -l format -d 'Set the display format.' -r -f -a "{long	,verbose	,single-column	,columns	,vertical	,across	,horizontal	,commas	}"
complete -c uu_ls -s T -l tabsize -d 'Assume tab stops at each COLS instead of 8 (unimplemented)' -r
complete -c uu_ls -l hyperlink -d 'hyperlink file names WHEN' -r -f -a "{always	,auto	,never	}"
complete -c uu_ls -l quoting-style -d 'Set quoting style.' -r -f -a "{literal	,shell	,shell-escape	,shell-always	,shell-escape-always	,c	,escape	}"
complete -c uu_ls -l time -d 'Show time in <field>:
	access time (-u): atime, access, use;
	change time (-t): ctime, status.
	birth time: birth, creation;' -r -f -a "{atime	,ctime	,birth	}"
complete -c uu_ls -l hide -d 'do not list implied entries matching shell PATTERN (overridden by -a or -A)' -r
complete -c uu_ls -s I -l ignore -d 'do not list implied entries matching shell PATTERN' -r
complete -c uu_ls -l sort -d 'Sort by <field>: name, none (-U), time (-t), size (-S), extension (-X) or width' -r -f -a "{name	,none	,time	,size	,version	,extension	,width	}"
complete -c uu_ls -l block-size -d 'scale sizes by BLOCK_SIZE when printing them' -r
complete -c uu_ls -s w -l width -d 'Assume that the terminal is COLS columns wide.' -r
complete -c uu_ls -l color -d 'Color output based on file type.' -r -f -a "{always	,auto	,never	}"
complete -c uu_ls -l indicator-style -d 'Append indicator with style WORD to entry names: none (default),  slash (-p), file-type (--file-type), classify (-F)' -r -f -a "{none	,slash	,file-type	,classify	}"
complete -c uu_ls -s F -l classify -d 'Append a character to each file name indicating the file type. Also, for regular files that are executable, append \'*\'. The file type indicators are \'/\' for directories, \'@\' for symbolic links, \'|\' for FIFOs, \'=\' for sockets, \'>\' for doors, and nothing for regular files. when may be omitted, or one of:
	none - Do not classify. This is the default.
	auto - Only classify if standard output is a terminal.
	always - Always classify.
Specifying --classify and no when is equivalent to --classify=always. This will not follow symbolic links listed on the command line unless the --dereference-command-line (-H), --dereference (-L), or --dereference-command-line-symlink-to-dir options are specified.' -r -f -a "{always	,auto	,never	}"
complete -c uu_ls -l time-style -d 'time/date format with -l; see TIME_STYLE below' -r
complete -c uu_ls -l help -d 'Print help information.'
complete -c uu_ls -s C -d 'Display the files in columns.'
complete -c uu_ls -s l -l long -d 'Display detailed information.'
complete -c uu_ls -s x -d 'List entries in rows instead of in columns.'
complete -c uu_ls -s m -d 'List entries separated by commas.'
complete -c uu_ls -l zero -d 'List entries separated by ASCII NUL characters.'
complete -c uu_ls -s D -l dired -d 'generate output designed for Emacs\' dired (Directory Editor) mode'
complete -c uu_ls -s 1 -d 'List one file per line.'
complete -c uu_ls -s o -d 'Long format without group information. Identical to --format=long with --no-group.'
complete -c uu_ls -s g -d 'Long format without owner information.'
complete -c uu_ls -s n -l numeric-uid-gid -d '-l with numeric UIDs and GIDs.'
complete -c uu_ls -s N -l literal -d 'Use literal quoting style. Equivalent to `--quoting-style=literal`'
complete -c uu_ls -s b -l escape -d 'Use escape quoting style. Equivalent to `--quoting-style=escape`'
complete -c uu_ls -s Q -l quote-name -d 'Use C quoting style. Equivalent to `--quoting-style=c`'
complete -c uu_ls -s q -l hide-control-chars -d 'Replace control characters with \'?\' if they are not escaped.'
complete -c uu_ls -l show-control-chars -d 'Show control characters \'as is\' if they are not escaped.'
complete -c uu_ls -s c -d 'If the long listing format (e.g., -l, -o) is being used, print the status change time (the \'ctime\' in the inode) instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the status change time.'
complete -c uu_ls -s u -d 'If the long listing format (e.g., -l, -o) is being used, print the status access time instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the access time.'
complete -c uu_ls -s B -l ignore-backups -d 'Ignore entries which end with ~.'
complete -c uu_ls -s S -d 'Sort by file size, largest first.'
complete -c uu_ls -s t -d 'Sort by modification time (the \'mtime\' in the inode), newest first.'
complete -c uu_ls -s v -d 'Natural sort of (version) numbers in the filenames.'
complete -c uu_ls -s X -d 'Sort alphabetically by entry extension.'
complete -c uu_ls -s U -d 'Do not sort; list the files in whatever order they are stored in the directory.  This is especially useful when listing very large directories, since not doing any sorting can be noticeably faster.'
complete -c uu_ls -s L -l dereference -d 'When showing file information for a symbolic link, show information for the file the link references rather than the link itself.'
complete -c uu_ls -l dereference-command-line-symlink-to-dir -d 'Do not follow symlinks except when they link to directories and are given as command line arguments.'
complete -c uu_ls -s H -l dereference-command-line -d 'Do not follow symlinks except when given as command line arguments.'
complete -c uu_ls -s G -l no-group -d 'Do not show group in long format.'
complete -c uu_ls -l author -d 'Show author in long format. On the supported platforms, the author always matches the file owner.'
complete -c uu_ls -s a -l all -d 'Do not ignore hidden files (files with names that start with \'.\').'
complete -c uu_ls -s A -l almost-all -d 'In a directory, do not ignore all file names that start with \'.\', only ignore \'.\' and \'..\'.'
complete -c uu_ls -s d -l directory -d 'Only list the names of directories, rather than listing directory contents. This will not follow symbolic links unless one of `--dereference-command-line (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is specified.'
complete -c uu_ls -s h -l human-readable -d 'Print human readable file sizes (e.g. 1K 234M 56G).'
complete -c uu_ls -s k -l kibibytes -d 'default to 1024-byte blocks for file system usage; used only with -s and per directory totals'
complete -c uu_ls -l si -d 'Print human readable file sizes using powers of 1000 instead of 1024.'
complete -c uu_ls -s i -l inode -d 'print the index number of each file'
complete -c uu_ls -s r -l reverse -d 'Reverse whatever the sorting method is e.g., list files in reverse alphabetical order, youngest first, smallest first, or whatever.'
complete -c uu_ls -s R -l recursive -d 'List the contents of all directories recursively.'
complete -c uu_ls -s s -l size -d 'print the allocated size of each file, in blocks'
complete -c uu_ls -l file-type -d 'Same as --classify, but do not append \'*\''
complete -c uu_ls -s p -d 'Append / indicator to directories.'
complete -c uu_ls -l full-time -d 'like -l --time-style=full-iso'
complete -c uu_ls -s Z -l context -d 'print any security context of each file'
complete -c uu_ls -l group-directories-first -d 'group directories before files; can be augmented with a --sort option, but any use of --sort=none (-U) disables grouping'
complete -c uu_ls -s V -l version -d 'Print version'
