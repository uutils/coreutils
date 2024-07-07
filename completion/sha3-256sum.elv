
use builtin;
use str;

set edit:completion:arg-completer[sha3-256sum] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'sha3-256sum'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'sha3-256sum'= {
            cand -b 'read in binary mode'
            cand --binary 'read in binary mode'
            cand -c 'read hashsums from the FILEs and check them'
            cand --check 'read hashsums from the FILEs and check them'
            cand --tag 'create a BSD-style checksum'
            cand -t 'read in text mode (default)'
            cand --text 'read in text mode (default)'
            cand -q 'don''t print OK for each successfully verified file'
            cand --quiet 'don''t print OK for each successfully verified file'
            cand -s 'don''t output anything, status code shows success'
            cand --status 'don''t output anything, status code shows success'
            cand --strict 'exit non-zero for improperly formatted checksum lines'
            cand --ignore-missing 'don''t fail or report status for missing files'
            cand -w 'warn about improperly formatted checksum lines'
            cand --warn 'warn about improperly formatted checksum lines'
            cand -z 'end each output line with NUL, not newline'
            cand --zero 'end each output line with NUL, not newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
