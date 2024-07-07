
use builtin;
use str;

set edit:completion:arg-completer[uu_chcon] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_chcon'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_chcon'= {
            cand --reference 'Use security context of RFILE, rather than specifying a CONTEXT value.'
            cand -u 'Set user USER in the target security context.'
            cand --user 'Set user USER in the target security context.'
            cand -r 'Set role ROLE in the target security context.'
            cand --role 'Set role ROLE in the target security context.'
            cand -t 'Set type TYPE in the target security context.'
            cand --type 'Set type TYPE in the target security context.'
            cand -l 'Set range RANGE in the target security context.'
            cand --range 'Set range RANGE in the target security context.'
            cand --help 'Print help information.'
            cand --dereference 'Affect the referent of each symbolic link (this is the default), rather than the symbolic link itself.'
            cand -h 'Affect symbolic links instead of any referenced file.'
            cand --no-dereference 'Affect symbolic links instead of any referenced file.'
            cand --preserve-root 'Fail to operate recursively on ''/''.'
            cand --no-preserve-root 'Do not treat ''/'' specially (the default).'
            cand -R 'Operate on files and directories recursively.'
            cand --recursive 'Operate on files and directories recursively.'
            cand -H 'If a command line argument is a symbolic link to a directory, traverse it. Only valid when -R is specified.'
            cand -L 'Traverse every symbolic link to a directory encountered. Only valid when -R is specified.'
            cand -P 'Do not traverse any symbolic links (default). Only valid when -R is specified.'
            cand -v 'Output a diagnostic for every file processed.'
            cand --verbose 'Output a diagnostic for every file processed.'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
