
use builtin;
use str;

set edit:completion:arg-completer[uu_cat] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_cat'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_cat'= {
            cand -A 'equivalent to -vET'
            cand --show-all 'equivalent to -vET'
            cand -b 'number nonempty output lines, overrides -n'
            cand --number-nonblank 'number nonempty output lines, overrides -n'
            cand -e 'equivalent to -vE'
            cand -E 'display $ at end of each line'
            cand --show-ends 'display $ at end of each line'
            cand -n 'number all output lines'
            cand --number 'number all output lines'
            cand -s 'suppress repeated empty output lines'
            cand --squeeze-blank 'suppress repeated empty output lines'
            cand -t 'equivalent to -vT'
            cand -T 'display TAB characters at ^I'
            cand --show-tabs 'display TAB characters at ^I'
            cand -v 'use ^ and M- notation, except for LF (\n) and TAB (\t)'
            cand --show-nonprinting 'use ^ and M- notation, except for LF (\n) and TAB (\t)'
            cand -u '(ignored)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
