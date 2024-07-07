
use builtin;
use str;

set edit:completion:arg-completer[uu_od] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_od'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_od'= {
            cand -A 'Select the base in which file offsets are printed.'
            cand --address-radix 'Select the base in which file offsets are printed.'
            cand -j 'Skip bytes input bytes before formatting and writing.'
            cand --skip-bytes 'Skip bytes input bytes before formatting and writing.'
            cand -N 'limit dump to BYTES input bytes'
            cand --read-bytes 'limit dump to BYTES input bytes'
            cand --endian 'byte order to use for multi-byte formats'
            cand -S 'NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when BYTES is not specified.'
            cand --strings 'NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when BYTES is not specified.'
            cand -t 'select output format or formats'
            cand --format 'select output format or formats'
            cand -w 'output BYTES bytes per output line. 32 is implied when BYTES is not specified.'
            cand --width 'output BYTES bytes per output line. 32 is implied when BYTES is not specified.'
            cand --help 'Print help information.'
            cand -a 'named characters, ignoring high-order bit'
            cand -b 'octal bytes'
            cand -c 'ASCII characters or backslash escapes'
            cand -d 'unsigned decimal 2-byte units'
            cand -D 'unsigned decimal 4-byte units'
            cand -o 'octal 2-byte units'
            cand -I 'decimal 8-byte units'
            cand -L 'decimal 8-byte units'
            cand -i 'decimal 4-byte units'
            cand -l 'decimal 8-byte units'
            cand -x 'hexadecimal 2-byte units'
            cand -h 'hexadecimal 2-byte units'
            cand -O 'octal 4-byte units'
            cand -s 'decimal 2-byte units'
            cand -X 'hexadecimal 4-byte units'
            cand -H 'hexadecimal 4-byte units'
            cand -e 'floating point double precision (64-bit) units'
            cand -f 'floating point double precision (32-bit) units'
            cand -F 'floating point double precision (64-bit) units'
            cand -v 'do not use * to mark line suppression'
            cand --output-duplicates 'do not use * to mark line suppression'
            cand --traditional 'compatibility mode with one input, offset and label.'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
