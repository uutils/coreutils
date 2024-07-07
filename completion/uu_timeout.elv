
use builtin;
use str;

set edit:completion:arg-completer[uu_timeout] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_timeout'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_timeout'= {
            cand -k 'also send a KILL signal if COMMAND is still running this long after the initial signal was sent'
            cand --kill-after 'also send a KILL signal if COMMAND is still running this long after the initial signal was sent'
            cand -s 'specify the signal to be sent on timeout; SIGNAL may be a name like ''HUP'' or a number; see ''kill -l'' for a list of signals'
            cand --signal 'specify the signal to be sent on timeout; SIGNAL may be a name like ''HUP'' or a number; see ''kill -l'' for a list of signals'
            cand --foreground 'when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out'
            cand --preserve-status 'exit with the same status as COMMAND, even when the command times out'
            cand -v 'diagnose to stderr any signal sent upon timeout'
            cand --verbose 'diagnose to stderr any signal sent upon timeout'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
