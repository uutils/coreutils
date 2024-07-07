
use builtin;
use str;

set edit:completion:arg-completer[uu_pathchk] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_pathchk'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_pathchk'= {
            cand -p 'check for most POSIX systems'
            cand -P 'check for empty names and leading "-"'
            cand --portability 'check for all POSIX systems (equivalent to -p -P)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
