
use builtin;
use str;

set edit:completion:arg-completer[uu_tee] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_tee'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_tee'= {
            cand --output-error 'set write error behavior'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -a 'append to the given FILEs, do not overwrite'
            cand --append 'append to the given FILEs, do not overwrite'
            cand -i 'ignore interrupt signals (ignored on non-Unix platforms)'
            cand --ignore-interrupts 'ignore interrupt signals (ignored on non-Unix platforms)'
            cand -p 'set write error behavior (ignored on non-Unix platforms)'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
