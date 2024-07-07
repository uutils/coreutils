
use builtin;
use str;

set edit:completion:arg-completer[uu_dircolors] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_dircolors'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_dircolors'= {
            cand -b 'output Bourne shell code to set LS_COLORS'
            cand --sh 'output Bourne shell code to set LS_COLORS'
            cand --bourne-shell 'output Bourne shell code to set LS_COLORS'
            cand -c 'output C shell code to set LS_COLORS'
            cand --csh 'output C shell code to set LS_COLORS'
            cand --c-shell 'output C shell code to set LS_COLORS'
            cand -p 'print the byte counts'
            cand --print-database 'print the byte counts'
            cand --print-ls-colors 'output fully escaped colors for display'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
