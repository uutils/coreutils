
use builtin;
use str;

set edit:completion:arg-completer[uu_ls] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_ls'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_ls'= {
            cand --format 'Set the display format.'
            cand -T 'Assume tab stops at each COLS instead of 8 (unimplemented)'
            cand --tabsize 'Assume tab stops at each COLS instead of 8 (unimplemented)'
            cand --hyperlink 'hyperlink file names WHEN'
            cand --quoting-style 'Set quoting style.'
            cand --time 'Show time in <field>:
	access time (-u): atime, access, use;
	change time (-t): ctime, status.
	birth time: birth, creation;'
            cand --hide 'do not list implied entries matching shell PATTERN (overridden by -a or -A)'
            cand -I 'do not list implied entries matching shell PATTERN'
            cand --ignore 'do not list implied entries matching shell PATTERN'
            cand --sort 'Sort by <field>: name, none (-U), time (-t), size (-S), extension (-X) or width'
            cand --block-size 'scale sizes by BLOCK_SIZE when printing them'
            cand -w 'Assume that the terminal is COLS columns wide.'
            cand --width 'Assume that the terminal is COLS columns wide.'
            cand --color 'Color output based on file type.'
            cand --indicator-style 'Append indicator with style WORD to entry names: none (default),  slash (-p), file-type (--file-type), classify (-F)'
            cand -F 'Append a character to each file name indicating the file type. Also, for regular files that are executable, append ''*''. The file type indicators are ''/'' for directories, ''@'' for symbolic links, ''|'' for FIFOs, ''='' for sockets, ''>'' for doors, and nothing for regular files. when may be omitted, or one of:
	none - Do not classify. This is the default.
	auto - Only classify if standard output is a terminal.
	always - Always classify.
Specifying --classify and no when is equivalent to --classify=always. This will not follow symbolic links listed on the command line unless the --dereference-command-line (-H), --dereference (-L), or --dereference-command-line-symlink-to-dir options are specified.'
            cand --classify 'Append a character to each file name indicating the file type. Also, for regular files that are executable, append ''*''. The file type indicators are ''/'' for directories, ''@'' for symbolic links, ''|'' for FIFOs, ''='' for sockets, ''>'' for doors, and nothing for regular files. when may be omitted, or one of:
	none - Do not classify. This is the default.
	auto - Only classify if standard output is a terminal.
	always - Always classify.
Specifying --classify and no when is equivalent to --classify=always. This will not follow symbolic links listed on the command line unless the --dereference-command-line (-H), --dereference (-L), or --dereference-command-line-symlink-to-dir options are specified.'
            cand --time-style 'time/date format with -l; see TIME_STYLE below'
            cand --help 'Print help information.'
            cand -C 'Display the files in columns.'
            cand -l 'Display detailed information.'
            cand --long 'Display detailed information.'
            cand -x 'List entries in rows instead of in columns.'
            cand -m 'List entries separated by commas.'
            cand --zero 'List entries separated by ASCII NUL characters.'
            cand -D 'generate output designed for Emacs'' dired (Directory Editor) mode'
            cand --dired 'generate output designed for Emacs'' dired (Directory Editor) mode'
            cand -1 'List one file per line.'
            cand -o 'Long format without group information. Identical to --format=long with --no-group.'
            cand -g 'Long format without owner information.'
            cand -n '-l with numeric UIDs and GIDs.'
            cand --numeric-uid-gid '-l with numeric UIDs and GIDs.'
            cand -N 'Use literal quoting style. Equivalent to `--quoting-style=literal`'
            cand --literal 'Use literal quoting style. Equivalent to `--quoting-style=literal`'
            cand -b 'Use escape quoting style. Equivalent to `--quoting-style=escape`'
            cand --escape 'Use escape quoting style. Equivalent to `--quoting-style=escape`'
            cand -Q 'Use C quoting style. Equivalent to `--quoting-style=c`'
            cand --quote-name 'Use C quoting style. Equivalent to `--quoting-style=c`'
            cand -q 'Replace control characters with ''?'' if they are not escaped.'
            cand --hide-control-chars 'Replace control characters with ''?'' if they are not escaped.'
            cand --show-control-chars 'Show control characters ''as is'' if they are not escaped.'
            cand -c 'If the long listing format (e.g., -l, -o) is being used, print the status change time (the ''ctime'' in the inode) instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the status change time.'
            cand -u 'If the long listing format (e.g., -l, -o) is being used, print the status access time instead of the modification time. When explicitly sorting by time (--sort=time or -t) or when not using a long listing format, sort according to the access time.'
            cand -B 'Ignore entries which end with ~.'
            cand --ignore-backups 'Ignore entries which end with ~.'
            cand -S 'Sort by file size, largest first.'
            cand -t 'Sort by modification time (the ''mtime'' in the inode), newest first.'
            cand -v 'Natural sort of (version) numbers in the filenames.'
            cand -X 'Sort alphabetically by entry extension.'
            cand -U 'Do not sort; list the files in whatever order they are stored in the directory.  This is especially useful when listing very large directories, since not doing any sorting can be noticeably faster.'
            cand -L 'When showing file information for a symbolic link, show information for the file the link references rather than the link itself.'
            cand --dereference 'When showing file information for a symbolic link, show information for the file the link references rather than the link itself.'
            cand --dereference-command-line-symlink-to-dir 'Do not follow symlinks except when they link to directories and are given as command line arguments.'
            cand -H 'Do not follow symlinks except when given as command line arguments.'
            cand --dereference-command-line 'Do not follow symlinks except when given as command line arguments.'
            cand -G 'Do not show group in long format.'
            cand --no-group 'Do not show group in long format.'
            cand --author 'Show author in long format. On the supported platforms, the author always matches the file owner.'
            cand -a 'Do not ignore hidden files (files with names that start with ''.'').'
            cand --all 'Do not ignore hidden files (files with names that start with ''.'').'
            cand -A 'In a directory, do not ignore all file names that start with ''.'', only ignore ''.'' and ''..''.'
            cand --almost-all 'In a directory, do not ignore all file names that start with ''.'', only ignore ''.'' and ''..''.'
            cand -d 'Only list the names of directories, rather than listing directory contents. This will not follow symbolic links unless one of `--dereference-command-line (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is specified.'
            cand --directory 'Only list the names of directories, rather than listing directory contents. This will not follow symbolic links unless one of `--dereference-command-line (-H)`, `--dereference (-L)`, or `--dereference-command-line-symlink-to-dir` is specified.'
            cand -h 'Print human readable file sizes (e.g. 1K 234M 56G).'
            cand --human-readable 'Print human readable file sizes (e.g. 1K 234M 56G).'
            cand -k 'default to 1024-byte blocks for file system usage; used only with -s and per directory totals'
            cand --kibibytes 'default to 1024-byte blocks for file system usage; used only with -s and per directory totals'
            cand --si 'Print human readable file sizes using powers of 1000 instead of 1024.'
            cand -i 'print the index number of each file'
            cand --inode 'print the index number of each file'
            cand -r 'Reverse whatever the sorting method is e.g., list files in reverse alphabetical order, youngest first, smallest first, or whatever.'
            cand --reverse 'Reverse whatever the sorting method is e.g., list files in reverse alphabetical order, youngest first, smallest first, or whatever.'
            cand -R 'List the contents of all directories recursively.'
            cand --recursive 'List the contents of all directories recursively.'
            cand -s 'print the allocated size of each file, in blocks'
            cand --size 'print the allocated size of each file, in blocks'
            cand --file-type 'Same as --classify, but do not append ''*'''
            cand -p 'Append / indicator to directories.'
            cand --full-time 'like -l --time-style=full-iso'
            cand -Z 'print any security context of each file'
            cand --context 'print any security context of each file'
            cand --group-directories-first 'group directories before files; can be augmented with a --sort option, but any use of --sort=none (-U) disables grouping'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
