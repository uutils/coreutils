
use builtin;
use str;

set edit:completion:arg-completer[uu_expr] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_expr'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_expr'= {
            cand --version 'output version information and exit'
            cand --help 'display this help and exit'
        }
    ]
    $completions[$command]
}
