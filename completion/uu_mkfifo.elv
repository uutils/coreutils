
use builtin;
use str;

set edit:completion:arg-completer[uu_mkfifo] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_mkfifo'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_mkfifo'= {
            cand -m 'file permissions for the fifo'
            cand --mode 'file permissions for the fifo'
            cand --context 'like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX'
            cand -Z 'set the SELinux security context to default type'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
