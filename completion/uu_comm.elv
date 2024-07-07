
use builtin;
use str;

set edit:completion:arg-completer[uu_comm] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_comm'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_comm'= {
            cand --output-delimiter 'separate columns with STR'
            cand -1 'suppress column 1 (lines unique to FILE1)'
            cand -2 'suppress column 2 (lines unique to FILE2)'
            cand -3 'suppress column 3 (lines that appear in both files)'
            cand -z 'line delimiter is NUL, not newline'
            cand --zero-terminated 'line delimiter is NUL, not newline'
            cand --total 'output a summary'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
