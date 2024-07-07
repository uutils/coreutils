
use builtin;
use str;

set edit:completion:arg-completer[uu_rmdir] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_rmdir'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_rmdir'= {
            cand --ignore-fail-on-non-empty 'ignore each failure that is solely because a directory is non-empty'
            cand -p 'remove DIRECTORY and its ancestors; e.g.,
                  ''rmdir -p a/b/c'' is similar to rmdir a/b/c a/b a'
            cand --parents 'remove DIRECTORY and its ancestors; e.g.,
                  ''rmdir -p a/b/c'' is similar to rmdir a/b/c a/b a'
            cand -v 'output a diagnostic for every directory processed'
            cand --verbose 'output a diagnostic for every directory processed'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
