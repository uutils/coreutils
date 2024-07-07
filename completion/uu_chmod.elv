
use builtin;
use str;

set edit:completion:arg-completer[uu_chmod] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_chmod'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_chmod'= {
            cand --reference 'use RFILE''s mode instead of MODE values'
            cand -c 'like verbose but report only when a change is made'
            cand --changes 'like verbose but report only when a change is made'
            cand -f 'suppress most error messages'
            cand --quiet 'suppress most error messages'
            cand --silent 'suppress most error messages'
            cand -v 'output a diagnostic for every file processed'
            cand --verbose 'output a diagnostic for every file processed'
            cand --no-preserve-root 'do not treat ''/'' specially (the default)'
            cand --preserve-root 'fail to operate recursively on ''/'''
            cand -R 'change files and directories recursively'
            cand --recursive 'change files and directories recursively'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
