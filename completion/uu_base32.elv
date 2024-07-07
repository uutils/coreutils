
use builtin;
use str;

set edit:completion:arg-completer[uu_base32] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_base32'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_base32'= {
            cand -w 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)'
            cand --wrap 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)'
            cand -d 'decode data'
            cand --decode 'decode data'
            cand -i 'when decoding, ignore non-alphabetic characters'
            cand --ignore-garbage 'when decoding, ignore non-alphabetic characters'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
