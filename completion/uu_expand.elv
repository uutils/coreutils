
use builtin;
use str;

set edit:completion:arg-completer[uu_expand] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_expand'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_expand'= {
            cand -t 'have tabs N characters apart, not 8 or use comma separated list of explicit tab positions'
            cand --tabs 'have tabs N characters apart, not 8 or use comma separated list of explicit tab positions'
            cand -i 'do not convert tabs after non blanks'
            cand --initial 'do not convert tabs after non blanks'
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
