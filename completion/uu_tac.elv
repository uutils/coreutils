
use builtin;
use str;

set edit:completion:arg-completer[uu_tac] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_tac'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_tac'= {
            cand -s 'use STRING as the separator instead of newline'
            cand --separator 'use STRING as the separator instead of newline'
            cand -b 'attach the separator before instead of after'
            cand --before 'attach the separator before instead of after'
            cand -r 'interpret the sequence as a regular expression'
            cand --regex 'interpret the sequence as a regular expression'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
