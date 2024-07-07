
use builtin;
use str;

set edit:completion:arg-completer[uu_factor] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_factor'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_factor'= {
            cand -h 'Print factors in the form p^e'
            cand --exponents 'Print factors in the form p^e'
            cand --help 'Print help information.'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
