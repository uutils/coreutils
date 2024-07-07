
use builtin;
use str;

set edit:completion:arg-completer[uu_kill] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_kill'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_kill'= {
            cand -s 'Sends given signal instead of SIGTERM'
            cand --signal 'Sends given signal instead of SIGTERM'
            cand -l 'Lists signals'
            cand --list 'Lists signals'
            cand -t 'Lists table of signals'
            cand --table 'Lists table of signals'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
