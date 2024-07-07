
use builtin;
use str;

set edit:completion:arg-completer[uu_stty] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_stty'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_stty'= {
            cand -F 'open and use the specified DEVICE instead of stdin'
            cand --file 'open and use the specified DEVICE instead of stdin'
            cand -a 'print all current settings in human-readable form'
            cand --all 'print all current settings in human-readable form'
            cand -g 'print all current settings in a stty-readable form'
            cand --save 'print all current settings in a stty-readable form'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
