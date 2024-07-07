
use builtin;
use str;

set edit:completion:arg-completer[uu_fold] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_fold'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_fold'= {
            cand -w 'set WIDTH as the maximum line width rather than 80'
            cand --width 'set WIDTH as the maximum line width rather than 80'
            cand -b 'count using bytes rather than columns (meaning control characters such as newline are not treated specially)'
            cand --bytes 'count using bytes rather than columns (meaning control characters such as newline are not treated specially)'
            cand -s 'break lines at word boundaries rather than a hard cut-off'
            cand --spaces 'break lines at word boundaries rather than a hard cut-off'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
