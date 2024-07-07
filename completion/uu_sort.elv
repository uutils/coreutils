
use builtin;
use str;

set edit:completion:arg-completer[uu_sort] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_sort'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_sort'= {
            cand --sort 'sort'
            cand -c 'check for sorted input; do not sort'
            cand --check 'check for sorted input; do not sort'
            cand -o 'write output to FILENAME instead of stdout'
            cand --output 'write output to FILENAME instead of stdout'
            cand -k 'sort by a key'
            cand --key 'sort by a key'
            cand -t 'custom separator for -k'
            cand --field-separator 'custom separator for -k'
            cand --parallel 'change the number of threads running concurrently to NUM_THREADS'
            cand -S 'sets the maximum SIZE of each segment in number of sorted items'
            cand --buffer-size 'sets the maximum SIZE of each segment in number of sorted items'
            cand -T 'use DIR for temporaries, not $TMPDIR or /tmp'
            cand --temporary-directory 'use DIR for temporaries, not $TMPDIR or /tmp'
            cand --compress-program 'compress temporary files with PROG, decompress with PROG -d; PROG has to take input from stdin and output to stdout'
            cand --batch-size 'Merge at most N_MERGE inputs at once.'
            cand --files0-from 'read input from the files specified by NUL-terminated NUL_FILES'
            cand --help 'Print help information.'
            cand --version 'Print version information.'
            cand -h 'compare according to human readable sizes, eg 1M > 100k'
            cand --human-numeric-sort 'compare according to human readable sizes, eg 1M > 100k'
            cand -M 'compare according to month name abbreviation'
            cand --month-sort 'compare according to month name abbreviation'
            cand -n 'compare according to string numerical value'
            cand --numeric-sort 'compare according to string numerical value'
            cand -g 'compare according to string general numerical value'
            cand --general-numeric-sort 'compare according to string general numerical value'
            cand -V 'Sort by SemVer version number, eg 1.12.2 > 1.1.2'
            cand --version-sort 'Sort by SemVer version number, eg 1.12.2 > 1.1.2'
            cand -R 'shuffle in random order'
            cand --random-sort 'shuffle in random order'
            cand -d 'consider only blanks and alphanumeric characters'
            cand --dictionary-order 'consider only blanks and alphanumeric characters'
            cand -m 'merge already sorted files; do not sort'
            cand --merge 'merge already sorted files; do not sort'
            cand -C 'exit successfully if the given file is already sorted, and exit with status 1 otherwise.'
            cand --check-silent 'exit successfully if the given file is already sorted, and exit with status 1 otherwise.'
            cand -f 'fold lower case to upper case characters'
            cand --ignore-case 'fold lower case to upper case characters'
            cand -i 'ignore nonprinting characters'
            cand --ignore-nonprinting 'ignore nonprinting characters'
            cand -b 'ignore leading blanks when finding sort keys in each line'
            cand --ignore-leading-blanks 'ignore leading blanks when finding sort keys in each line'
            cand -r 'reverse the output'
            cand --reverse 'reverse the output'
            cand -s 'stabilize sort by disabling last-resort comparison'
            cand --stable 'stabilize sort by disabling last-resort comparison'
            cand -u 'output only the first of an equal run'
            cand --unique 'output only the first of an equal run'
            cand -z 'line delimiter is NUL, not newline'
            cand --zero-terminated 'line delimiter is NUL, not newline'
            cand --debug 'underline the parts of the line that are actually used for sorting'
        }
    ]
    $completions[$command]
}
