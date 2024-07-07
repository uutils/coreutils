
use builtin;
use str;

set edit:completion:arg-completer[uu_shuf] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_shuf'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_shuf'= {
            cand -i 'treat each number LO through HI as an input line'
            cand --input-range 'treat each number LO through HI as an input line'
            cand -n 'output at most COUNT lines'
            cand --head-count 'output at most COUNT lines'
            cand -o 'write result to FILE instead of standard output'
            cand --output 'write result to FILE instead of standard output'
            cand --random-source 'get random bytes from FILE'
            cand -e 'treat each ARG as an input line'
            cand --echo 'treat each ARG as an input line'
            cand -r 'output lines can be repeated'
            cand --repeat 'output lines can be repeated'
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
