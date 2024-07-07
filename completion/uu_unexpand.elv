
use builtin;
use str;

set edit:completion:arg-completer[uu_unexpand] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_unexpand'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_unexpand'= {
            cand -t 'use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)'
            cand --tabs 'use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)'
            cand -a 'convert all blanks, instead of just initial blanks'
            cand --all 'convert all blanks, instead of just initial blanks'
            cand --first-only 'convert only leading sequences of blanks (overrides -a)'
            cand -U 'interpret input file as 8-bit ASCII rather than UTF-8'
            cand --no-utf8 'interpret input file as 8-bit ASCII rather than UTF-8'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
