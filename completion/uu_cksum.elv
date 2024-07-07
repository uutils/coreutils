
use builtin;
use str;

set edit:completion:arg-completer[uu_cksum] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_cksum'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_cksum'= {
            cand -a 'select the digest type to use. See DIGEST below'
            cand --algorithm 'select the digest type to use. See DIGEST below'
            cand -l 'digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8'
            cand --length 'digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8'
            cand --untagged 'create a reversed style checksum, without digest type'
            cand --tag 'create a BSD style checksum, undo --untagged (default)'
            cand --raw 'emit a raw binary digest, not hexadecimal'
            cand --strict 'exit non-zero for improperly formatted checksum lines'
            cand -c 'read hashsums from the FILEs and check them'
            cand --check 'read hashsums from the FILEs and check them'
            cand --base64 'emit a base64 digest, not hexadecimal'
            cand -t 't'
            cand --text 'text'
            cand -b 'b'
            cand --binary 'binary'
            cand -w 'warn about improperly formatted checksum lines'
            cand --warn 'warn about improperly formatted checksum lines'
            cand --status 'don''t output anything, status code shows success'
            cand --quiet 'don''t print OK for each successfully verified file'
            cand --ignore-missing 'don''t fail or report status for missing files'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
