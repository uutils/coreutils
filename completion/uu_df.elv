
use builtin;
use str;

set edit:completion:arg-completer[uu_df] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_df'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_df'= {
            cand -B 'scale sizes by SIZE before printing them; e.g.''-BM'' prints sizes in units of 1,048,576 bytes'
            cand --block-size 'scale sizes by SIZE before printing them; e.g.''-BM'' prints sizes in units of 1,048,576 bytes'
            cand --output 'use the output format defined by FIELD_LIST, or print all fields if FIELD_LIST is omitted.'
            cand -t 'limit listing to file systems of type TYPE'
            cand --type 'limit listing to file systems of type TYPE'
            cand -x 'limit listing to file systems not of type TYPE'
            cand --exclude-type 'limit listing to file systems not of type TYPE'
            cand --help 'Print help information.'
            cand -a 'include dummy file systems'
            cand --all 'include dummy file systems'
            cand --total 'produce a grand total'
            cand -h 'print sizes in human readable format (e.g., 1K 234M 2G)'
            cand --human-readable 'print sizes in human readable format (e.g., 1K 234M 2G)'
            cand -H 'likewise, but use powers of 1000 not 1024'
            cand --si 'likewise, but use powers of 1000 not 1024'
            cand -i 'list inode information instead of block usage'
            cand --inodes 'list inode information instead of block usage'
            cand -k 'like --block-size=1K'
            cand -l 'limit listing to local file systems'
            cand --local 'limit listing to local file systems'
            cand --no-sync 'do not invoke sync before getting usage info (default)'
            cand -P 'use the POSIX output format'
            cand --portability 'use the POSIX output format'
            cand --sync 'invoke sync before getting usage info (non-windows only)'
            cand -T 'print file system type'
            cand --print-type 'print file system type'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
