
use builtin;
use str;

set edit:completion:arg-completer[uu_numfmt] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_numfmt'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_numfmt'= {
            cand -d 'use X instead of whitespace for field delimiter'
            cand --delimiter 'use X instead of whitespace for field delimiter'
            cand --field 'replace the numbers in these input fields; see FIELDS below'
            cand --format 'use printf style floating-point FORMAT; see FORMAT below for details'
            cand --from 'auto-scale input numbers to UNITs; see UNIT below'
            cand --from-unit 'specify the input unit size'
            cand --to 'auto-scale output numbers to UNITs; see UNIT below'
            cand --to-unit 'the output unit size'
            cand --padding 'pad the output to N characters; positive N will right-align; negative N will left-align; padding is ignored if the output is wider than N; the default is to automatically pad if a whitespace is found'
            cand --header 'print (without converting) the first N header lines; N defaults to 1 if not specified'
            cand --round 'use METHOD for rounding when scaling'
            cand --suffix 'print SUFFIX after each formatted number, and accept inputs optionally ending with SUFFIX'
            cand --invalid 'set the failure mode for invalid input'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
