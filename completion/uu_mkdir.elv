
use builtin;
use str;

set edit:completion:arg-completer[uu_mkdir] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_mkdir'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_mkdir'= {
            cand -m 'set file mode (not implemented on windows)'
            cand --mode 'set file mode (not implemented on windows)'
            cand -p 'make parent directories as needed'
            cand --parents 'make parent directories as needed'
            cand -v 'print a message for each printed directory'
            cand --verbose 'print a message for each printed directory'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
