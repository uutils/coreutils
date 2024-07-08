
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_ls' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_ls'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'uu_ls' {
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'Set the display format.')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'Assume tab stops at each COLS instead of 8 (unimplemented)')
            [CompletionResult]::new('--tabsize', 'tabsize', [CompletionResultType]::ParameterName, 'Assume tab stops at each COLS instead of 8 (unimplemented)')
            [CompletionResult]::new('--hyperlink', 'hyperlink', [CompletionResultType]::ParameterName, 'hyperlink file names WHEN')
            [CompletionResult]::new('--quoting-style', 'quoting-style', [CompletionResultType]::ParameterName, 'Set quoting style.')
            [CompletionResult]::new('--time', 'time', [CompletionResultType]::ParameterName, 'Show time in <field>:
	access time (-u): atime, access, use;
	change time (-t): ctime, status.
	birth time: birth, creation;')
            [CompletionResult]::new('--hide', 'hide', [CompletionResultType]::ParameterName, 'do not list implied entries matching shell PATTERN (overridden by -a or -A)')
            [CompletionResult]::new('-I', 'I ', [CompletionResultType]::ParameterName, 'do not list implied entries matching shell PATTERN')
            [CompletionResult]::new('--ignore', 'ignore', [CompletionResultType]::ParameterName, 'do not list implied entries matching shell PATTERN')
            [CompletionResult]::new('--sort', 'sort', [CompletionResultType]::ParameterName, 'Sort by <field>: name, none (-U), time (-t), size (-S), extension (-X) or width')
            [CompletionResult]::new('--block-size', 'block-size', [CompletionResultType]::ParameterName, 'scale sizes by BLOCK_SIZE when printing them')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'Assume that the terminal is COLS columns wide.')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'Assume that the terminal is COLS columns wide.')
            [CompletionResult]::new('--color', 'color', [CompletionResultType]::ParameterName, 'Color output based on file type.')
            [CompletionResult]::new('--indicator-style', 'indicator-style', [CompletionResultType]::ParameterName, 'Append indicator with style WORD to entry names: none (default),  slash (-p), file-type (--file-type), classify (-F)')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'Append a character to each file name indicating the file type. Also, for regular files that are executable, append ''*''. The file type indicators are ''/'' for directories, ''@'' for symbolic links, ''|'' for FIFOs, ''='' for sockets, ''>'' for doors, and nothing for regular files. when may be omitted, or one of:
	none - Do not classify. This is the default.
	auto - Only classify if standard output is a terminal.
	always - Always classify.
Specifying --classify and no when is equivalent to --classify=always. This will not follow symbolic links listed on the command line unless the --dereference-command-line (-H), --dereference (-L), or --dereference-command-line-symlink-to-dir options are specified.')
            [CompletionResult]::new('--classify', 'classify', [CompletionResultType]::ParameterName, 'Append a character to each file name indicating the file type. Also, for regular files that are executable, append ''*''. The file type indicators are ''/'' for directories, ''@'' for symbolic links, ''|'' for FIFOs, ''='' for sockets, ''>'' for doors, and nothing for regular files. when may be omitted, or one of:
	none - Do not classify. This is the default.
	auto - Only classify if standard output is a terminal.
	always - Always classify.
Specifying --classify and no when is equivalent to --classify=always. This will not follow symbolic links listed on the command line unless the --dereference-command-line (-H), --dereference (-L), or --dereference-command-line-symlink-to-dir options are specified.')
            [CompletionResult]::new('--time-style', 'time-style', [CompletionResultType]::ParameterName, 'time/date format with -l; see TIME_STYLE below')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'Display the files in columns.')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Display detailed information.')
            [CompletionResult]::new('--long', 'long', [CompletionResultType]::ParameterName, 'Display detailed information.')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'List entries in rows instead of in columns.')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'List entries separated by commas.')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'List entries separated by ASCII NUL characters.')
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'generate output designed for Emacs'' dired (Directory Editor) mode')
            [CompletionResult]::new('--dired', 'dired', [CompletionResultType]::ParameterName, 'generate output designed for Emacs'' dired (Directory Editor) mode')
            [CompletionResult]::new('-1', '1', [CompletionResultType]::ParameterName, 'List one file per line.')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'Long format without group information. Identical to --format=long with --no-group.')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Long format without owner information.')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, '-l with numeric UIDs and GIDs.')
            [CompletionResult]::new('--numeric-uid-gid', 'numeric-uid-gid', [CompletionResultType]::ParameterName, '-l with numeric UIDs and GIDs.')
            [CompletionResult]::new('-N', 'N ', [CompletionResultType]::ParameterName, 'Use literal quoting style. Equivalent to `--quoting-style=literal`')
            [CompletionResult]::new('--literal', 'literal', [CompletionResultType]::ParameterName, 'Use literal quoting style. Equivalent to `--quoting-style=literal`')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'Use escape quoting style. Equivalent to `--quoting-style=escape`')
            [CompletionResult]::new('--escape', 'escape', [CompletionResultType]::ParameterName, 'Use escape quoting style. Equivalent to `--quoting-style=escape`')
            [CompletionResult]::new('-Q', 'Q ', [CompletionResultType]::ParameterName, 'Use C quoting style. Equivalent to `--quoting-style=c`')
            [CompletionResult]::new('--quote-name', 'quote-name', [CompletionResultType]::ParameterName, 'Use C quoting style. Equivalent to `--quoting-style=c`')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Replace control characters with ''?'' if they are not escaped.')
            [CompletionResult]::new('--hide-control-chars', 'hide-control-chars', [CompletionResultType]::ParameterName, 'Replace control characters with ''?'' if they are not escaped.')
            [CompletionResult]::new('--show-control-chars', 'show-control-chars', [CompletionResultType]::ParameterName, 'Show control characters ''as is'' if they are not escaped.')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'If the long listing format (e.g., -l, -o) is being used, print the status change time (the ''ctime'' in the inode) instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the status change time.')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'If the long listing format (e.g., -l, -o) is being used, print the status access time instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the access time.')
            [CompletionResult]::new('-B', 'B ', [CompletionResultType]::ParameterName, 'Ignore entries which end with ~.')
            [CompletionResult]::new('--ignore-backups', 'ignore-backups', [CompletionResultType]::ParameterName, 'Ignore entries which end with ~.')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'Sort by file size, largest first.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Sort by modification time (the ''mtime'' in the inode), newest first.')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Natural sort of (version) numbers in the filenames.')
            [CompletionResult]::new('-X', 'X ', [CompletionResultType]::ParameterName, 'Sort alphabetically by entry extension.')
            [CompletionResult]::new('-U', 'U ', [CompletionResultType]::ParameterName, 'Do not sort; list the files in whatever order they are stored in the directory.  This is especially useful when listing very large directories, since not doing any sorting can be noticeably faster.')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'When showing file information for a symbolic link, show information for the file the link references rather than the link itself.')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'When showing file information for a symbolic link, show information for the file the link references rather than the link itself.')
            [CompletionResult]::new('--dereference-command-line-symlink-to-dir', 'dereference-command-line-symlink-to-dir', [CompletionResultType]::ParameterName, 'Do not follow symlinks except when they link to directories and are given as command line arguments.')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'Do not follow symlinks except when given as command line arguments.')
            [CompletionResult]::new('--dereference-command-line', 'dereference-command-line', [CompletionResultType]::ParameterName, 'Do not follow symlinks except when given as command line arguments.')
            [CompletionResult]::new('-G', 'G ', [CompletionResultType]::ParameterName, 'Do not show group in long format.')
            [CompletionResult]::new('--no-group', 'no-group', [CompletionResultType]::ParameterName, 'Do not show group in long format.')
            [CompletionResult]::new('--author', 'author', [CompletionResultType]::ParameterName, 'Show author in long format. On the supported platforms, the author always matches the file owner.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Do not ignore hidden files (files with names that start with ''.'').')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'Do not ignore hidden files (files with names that start with ''.'').')
            [CompletionResult]::new('-A', 'A ', [CompletionResultType]::ParameterName, 'In a directory, do not ignore all file names that start with ''.'', only ignore ''.'' and ''..''.')
            [CompletionResult]::new('--almost-all', 'almost-all', [CompletionResultType]::ParameterName, 'In a directory, do not ignore all file names that start with ''.'', only ignore ''.'' and ''..''.')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Only list the names of directories, rather than listing directory contents. This will not follow symbolic links unless one of `--dereference-command-line (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is specified.')
            [CompletionResult]::new('--directory', 'directory', [CompletionResultType]::ParameterName, 'Only list the names of directories, rather than listing directory contents. This will not follow symbolic links unless one of `--dereference-command-line (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is specified.')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print human readable file sizes (e.g. 1K 234M 56G).')
            [CompletionResult]::new('--human-readable', 'human-readable', [CompletionResultType]::ParameterName, 'Print human readable file sizes (e.g. 1K 234M 56G).')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'default to 1024-byte blocks for file system usage; used only with -s and per directory totals')
            [CompletionResult]::new('--kibibytes', 'kibibytes', [CompletionResultType]::ParameterName, 'default to 1024-byte blocks for file system usage; used only with -s and per directory totals')
            [CompletionResult]::new('--si', 'si', [CompletionResultType]::ParameterName, 'Print human readable file sizes using powers of 1000 instead of 1024.')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'print the index number of each file')
            [CompletionResult]::new('--inode', 'inode', [CompletionResultType]::ParameterName, 'print the index number of each file')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Reverse whatever the sorting method is e.g., list files in reverse alphabetical order, youngest first, smallest first, or whatever.')
            [CompletionResult]::new('--reverse', 'reverse', [CompletionResultType]::ParameterName, 'Reverse whatever the sorting method is e.g., list files in reverse alphabetical order, youngest first, smallest first, or whatever.')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'List the contents of all directories recursively.')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'List the contents of all directories recursively.')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'print the allocated size of each file, in blocks')
            [CompletionResult]::new('--size', 'size', [CompletionResultType]::ParameterName, 'print the allocated size of each file, in blocks')
            [CompletionResult]::new('--file-type', 'file-type', [CompletionResultType]::ParameterName, 'Same as --classify, but do not append ''*''')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Append / indicator to directories.')
            [CompletionResult]::new('--full-time', 'full-time', [CompletionResultType]::ParameterName, 'like -l --time-style=full-iso')
            [CompletionResult]::new('-Z', 'Z ', [CompletionResultType]::ParameterName, 'print any security context of each file')
            [CompletionResult]::new('--context', 'context', [CompletionResultType]::ParameterName, 'print any security context of each file')
            [CompletionResult]::new('--group-directories-first', 'group-directories-first', [CompletionResultType]::ParameterName, 'group directories before files; can be augmented with a --sort option, but any use of --sort=none (-U) disables grouping')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
