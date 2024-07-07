
use builtin;
use str;

set edit:completion:arg-completer[uu_head] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_head'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_head'= {
            cand -c 'print the first NUM bytes of each file;
with the leading ''-'', print all but the last
NUM bytes of each file'
            cand --bytes 'print the first NUM bytes of each file;
with the leading ''-'', print all but the last
NUM bytes of each file'
            cand -n 'print the first NUM lines instead of the first 10;
with the leading ''-'', print all but the last
NUM lines of each file'
            cand --lines 'print the first NUM lines instead of the first 10;
with the leading ''-'', print all but the last
NUM lines of each file'
            cand -q 'never print headers giving file names'
            cand --quiet 'never print headers giving file names'
            cand --silent 'never print headers giving file names'
            cand -v 'always print headers giving file names'
            cand --verbose 'always print headers giving file names'
            cand --presume-input-pipe 'presume-input-pipe'
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
