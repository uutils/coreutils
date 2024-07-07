
use builtin;
use str;

set edit:completion:arg-completer[uu_du] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_du'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_du'= {
            cand -B 'scale sizes by SIZE before printing them. E.g., ''-BM'' prints sizes in units of 1,048,576 bytes. See SIZE format below.'
            cand --block-size 'scale sizes by SIZE before printing them. E.g., ''-BM'' prints sizes in units of 1,048,576 bytes. See SIZE format below.'
            cand -d 'print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize'
            cand --max-depth 'print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize'
            cand -t 'exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative'
            cand --threshold 'exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative'
            cand --exclude 'exclude files that match PATTERN'
            cand -X 'exclude files that match any pattern in FILE'
            cand --exclude-from 'exclude files that match any pattern in FILE'
            cand --files0-from 'summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input'
            cand --time 'show time of the last modification of any file in the directory, or any of its subdirectories. If WORD is given, show time as WORD instead of modification time: atime, access, use, ctime, status, birth or creation'
            cand --time-style 'show times using style STYLE: full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like ''date'''
            cand --help 'Print help information.'
            cand -a 'write counts for all files, not just directories'
            cand --all 'write counts for all files, not just directories'
            cand --apparent-size 'print apparent sizes, rather than disk usage although the apparent size is usually smaller, it may be larger due to holes in (''sparse'') files, internal fragmentation, indirect blocks, and the like'
            cand -b 'equivalent to ''--apparent-size --block-size=1'''
            cand --bytes 'equivalent to ''--apparent-size --block-size=1'''
            cand -c 'produce a grand total'
            cand --total 'produce a grand total'
            cand -h 'print sizes in human readable format (e.g., 1K 234M 2G)'
            cand --human-readable 'print sizes in human readable format (e.g., 1K 234M 2G)'
            cand --inodes 'list inode usage information instead of block usage like --block-size=1K'
            cand -k 'like --block-size=1K'
            cand -l 'count sizes many times if hard linked'
            cand --count-links 'count sizes many times if hard linked'
            cand -L 'follow all symbolic links'
            cand --dereference 'follow all symbolic links'
            cand -D 'follow only symlinks that are listed on the command line'
            cand -H 'follow only symlinks that are listed on the command line'
            cand --dereference-args 'follow only symlinks that are listed on the command line'
            cand -P 'don''t follow any symbolic links (this is the default)'
            cand --no-dereference 'don''t follow any symbolic links (this is the default)'
            cand -m 'like --block-size=1M'
            cand -0 'end each output line with 0 byte rather than newline'
            cand --null 'end each output line with 0 byte rather than newline'
            cand -S 'do not include size of subdirectories'
            cand --separate-dirs 'do not include size of subdirectories'
            cand -s 'display only a total for each argument'
            cand --summarize 'display only a total for each argument'
            cand --si 'like -h, but use powers of 1000 not 1024'
            cand -x 'skip directories on different file systems'
            cand --one-file-system 'skip directories on different file systems'
            cand -v 'verbose mode (option not present in GNU/Coreutils)'
            cand --verbose 'verbose mode (option not present in GNU/Coreutils)'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
