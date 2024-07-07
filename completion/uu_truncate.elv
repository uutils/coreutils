
use builtin;
use str;

set edit:completion:arg-completer[uu_truncate] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_truncate'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_truncate'= {
            cand -r 'base the size of each file on the size of RFILE'
            cand --reference 'base the size of each file on the size of RFILE'
            cand -s 'set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified'
            cand --size 'set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified'
            cand -o 'treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)'
            cand --io-blocks 'treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)'
            cand -c 'do not create files that do not exist'
            cand --no-create 'do not create files that do not exist'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
