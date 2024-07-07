
use builtin;
use str;

set edit:completion:arg-completer[uu_chgrp] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_chgrp'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_chgrp'= {
            cand --reference 'use RFILE''s group rather than specifying GROUP values'
            cand --help 'Print help information.'
            cand -c 'like verbose but report only when a change is made'
            cand --changes 'like verbose but report only when a change is made'
            cand -f 'f'
            cand --silent 'silent'
            cand --quiet 'suppress most error messages'
            cand -v 'output a diagnostic for every file processed'
            cand --verbose 'output a diagnostic for every file processed'
            cand --dereference 'dereference'
            cand -h 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)'
            cand --no-dereference 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)'
            cand --preserve-root 'fail to operate recursively on ''/'''
            cand --no-preserve-root 'do not treat ''/'' specially (the default)'
            cand -R 'operate on files and directories recursively'
            cand --recursive 'operate on files and directories recursively'
            cand -H 'if a command line argument is a symbolic link to a directory, traverse it'
            cand -P 'do not traverse any symbolic links (default)'
            cand -L 'traverse every symbolic link to a directory encountered'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
