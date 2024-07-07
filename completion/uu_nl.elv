
use builtin;
use str;

set edit:completion:arg-completer[uu_nl] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_nl'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_nl'= {
            cand -b 'use STYLE for numbering body lines'
            cand --body-numbering 'use STYLE for numbering body lines'
            cand -d 'use CC for separating logical pages'
            cand --section-delimiter 'use CC for separating logical pages'
            cand -f 'use STYLE for numbering footer lines'
            cand --footer-numbering 'use STYLE for numbering footer lines'
            cand -h 'use STYLE for numbering header lines'
            cand --header-numbering 'use STYLE for numbering header lines'
            cand -i 'line number increment at each line'
            cand --line-increment 'line number increment at each line'
            cand -l 'group of NUMBER empty lines counted as one'
            cand --join-blank-lines 'group of NUMBER empty lines counted as one'
            cand -n 'insert line numbers according to FORMAT'
            cand --number-format 'insert line numbers according to FORMAT'
            cand -s 'add STRING after (possible) line number'
            cand --number-separator 'add STRING after (possible) line number'
            cand -v 'first line number on each logical page'
            cand --starting-line-number 'first line number on each logical page'
            cand -w 'use NUMBER columns for line numbers'
            cand --number-width 'use NUMBER columns for line numbers'
            cand --help 'Print help information.'
            cand -p 'do not reset line numbers at logical pages'
            cand --no-renumber 'do not reset line numbers at logical pages'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
