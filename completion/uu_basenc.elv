
use builtin;
use str;

set edit:completion:arg-completer[uu_basenc] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_basenc'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_basenc'= {
            cand -w 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)'
            cand --wrap 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)'
            cand -d 'decode data'
            cand --decode 'decode data'
            cand -i 'when decoding, ignore non-alphabetic characters'
            cand --ignore-garbage 'when decoding, ignore non-alphabetic characters'
            cand --base64 'same as ''base64'' program'
            cand --base64url 'file- and url-safe base64'
            cand --base32 'same as ''base32'' program'
            cand --base32hex 'extended hex alphabet base32'
            cand --base16 'hex encoding'
            cand --base2lsbf 'bit string with least significant bit (lsb) first'
            cand --base2msbf 'bit string with most significant bit (msb) first'
            cand --z85 'ascii85-like encoding;
when encoding, input length must be a multiple of 4;
when decoding, input length must be a multiple of 5'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
