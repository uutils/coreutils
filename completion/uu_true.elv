
use builtin;
use str;

set edit:completion:arg-completer[uu_true] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_true'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_true'= {
            cand --help 'Print help information'
            cand --version 'Print version information'
        }
    ]
    $completions[$command]
}
