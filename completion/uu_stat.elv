
use builtin;
use str;

set edit:completion:arg-completer[uu_stat] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_stat'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_stat'= {
            cand -c 'use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT'
            cand --format 'use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT'
            cand --printf 'like --format, but interpret backslash escapes,
            and do not output a mandatory trailing newline;
            if you want a newline, include 
 in FORMAT'
            cand -L 'follow links'
            cand --dereference 'follow links'
            cand -f 'display file system status instead of file status'
            cand --file-system 'display file system status instead of file status'
            cand -t 'print the information in terse form'
            cand --terse 'print the information in terse form'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
