
use builtin;
use str;

set edit:completion:arg-completer[uu_stdbuf] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_stdbuf'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_stdbuf'= {
            cand -i 'adjust standard input stream buffering'
            cand --input 'adjust standard input stream buffering'
            cand -o 'adjust standard output stream buffering'
            cand --output 'adjust standard output stream buffering'
            cand -e 'adjust standard error stream buffering'
            cand --error 'adjust standard error stream buffering'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
