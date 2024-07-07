
use builtin;
use str;

set edit:completion:arg-completer[uu_seq] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_seq'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_seq'= {
            cand -s 'Separator character (defaults to \n)'
            cand --separator 'Separator character (defaults to \n)'
            cand -t 'Terminator character (defaults to \n)'
            cand --terminator 'Terminator character (defaults to \n)'
            cand -f 'use printf style floating-point FORMAT'
            cand --format 'use printf style floating-point FORMAT'
            cand -w 'Equalize widths of all numbers by padding with zeros'
            cand --equal-width 'Equalize widths of all numbers by padding with zeros'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
