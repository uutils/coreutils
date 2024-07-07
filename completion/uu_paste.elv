
use builtin;
use str;

set edit:completion:arg-completer[uu_paste] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_paste'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_paste'= {
            cand -d 'reuse characters from LIST instead of TABs'
            cand --delimiters 'reuse characters from LIST instead of TABs'
            cand -s 'paste one file at a time instead of in parallel'
            cand --serial 'paste one file at a time instead of in parallel'
            cand -z 'line delimiter is NUL, not newline'
            cand --zero-terminated 'line delimiter is NUL, not newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
