
use builtin;
use str;

set edit:completion:arg-completer[uu_env] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_env'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_env'= {
            cand -C 'change working directory to DIR'
            cand --chdir 'change working directory to DIR'
            cand -f 'read and set variables from a ".env"-style configuration file (prior to any unset and/or set)'
            cand --file 'read and set variables from a ".env"-style configuration file (prior to any unset and/or set)'
            cand -u 'remove variable from the environment'
            cand --unset 'remove variable from the environment'
            cand -S 'process and split S into separate arguments; used to pass multiple arguments on shebang lines'
            cand --split-string 'process and split S into separate arguments; used to pass multiple arguments on shebang lines'
            cand -a 'Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.'
            cand --argv0 'Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.'
            cand --ignore-signal 'set handling of SIG signal(s) to do nothing'
            cand -i 'start with an empty environment'
            cand --ignore-environment 'start with an empty environment'
            cand -0 'end each output line with a 0 byte rather than a newline (only valid when printing the environment)'
            cand --null 'end each output line with a 0 byte rather than a newline (only valid when printing the environment)'
            cand -v 'print verbose information for each processing step'
            cand --debug 'print verbose information for each processing step'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
