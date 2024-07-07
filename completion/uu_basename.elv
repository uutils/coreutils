
use builtin;
use str;

set edit:completion:arg-completer[uu_basename] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_basename'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_basename'= {
            cand -s 'remove a trailing SUFFIX; implies -a'
            cand --suffix 'remove a trailing SUFFIX; implies -a'
            cand -a 'support multiple arguments and treat each as a NAME'
            cand --multiple 'support multiple arguments and treat each as a NAME'
            cand -z 'end each output line with NUL, not newline'
            cand --zero 'end each output line with NUL, not newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
