
use builtin;
use str;

set edit:completion:arg-completer[uu_pwd] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_pwd'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_pwd'= {
            cand -L 'use PWD from environment, even if it contains symlinks'
            cand --logical 'use PWD from environment, even if it contains symlinks'
            cand -P 'avoid all symlinks'
            cand --physical 'avoid all symlinks'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
