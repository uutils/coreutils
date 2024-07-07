
use builtin;
use str;

set edit:completion:arg-completer[uu_sum] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_sum'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_sum'= {
            cand -r 'use the BSD sum algorithm, use 1K blocks (default)'
            cand -s 'use System V sum algorithm, use 512 bytes blocks'
            cand --sysv 'use System V sum algorithm, use 512 bytes blocks'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
