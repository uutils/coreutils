
use builtin;
use str;

set edit:completion:arg-completer[uu_runcon] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_runcon'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_runcon'= {
            cand -u 'Set user USER in the target security context.'
            cand --user 'Set user USER in the target security context.'
            cand -r 'Set role ROLE in the target security context.'
            cand --role 'Set role ROLE in the target security context.'
            cand -t 'Set type TYPE in the target security context.'
            cand --type 'Set type TYPE in the target security context.'
            cand -l 'Set range RANGE in the target security context.'
            cand --range 'Set range RANGE in the target security context.'
            cand -c 'Compute process transition context before modifying.'
            cand --compute 'Compute process transition context before modifying.'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
