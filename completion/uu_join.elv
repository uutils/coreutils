
use builtin;
use str;

set edit:completion:arg-completer[uu_join] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_join'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_join'= {
            cand -a 'also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2'
            cand -v 'like -a FILENUM, but suppress joined output lines'
            cand -e 'replace missing input fields with EMPTY'
            cand -j 'equivalent to ''-1 FIELD -2 FIELD'''
            cand -o 'obey FORMAT while constructing output line'
            cand -t 'use CHAR as input and output field separator'
            cand -1 'join on this FIELD of file 1'
            cand -2 'join on this FIELD of file 2'
            cand -i 'ignore differences in case when comparing fields'
            cand --ignore-case 'ignore differences in case when comparing fields'
            cand --check-order 'check that the input is correctly sorted, even if all input lines are pairable'
            cand --nocheck-order 'do not check that the input is correctly sorted'
            cand --header 'treat the first line in each file as field headers, print them without trying to pair them'
            cand -z 'line delimiter is NUL, not newline'
            cand --zero-terminated 'line delimiter is NUL, not newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
