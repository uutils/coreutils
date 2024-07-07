
use builtin;
use str;

set edit:completion:arg-completer[uu_tail] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_tail'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_tail'= {
            cand -c 'Number of bytes to print'
            cand --bytes 'Number of bytes to print'
            cand -f 'Print the file as it grows'
            cand --follow 'Print the file as it grows'
            cand -n 'Number of lines to print'
            cand --lines 'Number of lines to print'
            cand --pid 'With -f, terminate after process ID, PID dies'
            cand -s 'Number of seconds to sleep between polling the file when running with -f'
            cand --sleep-interval 'Number of seconds to sleep between polling the file when running with -f'
            cand --max-unchanged-stats 'Reopen a FILE which has not changed size after N (default 5) iterations to see if it has been unlinked or renamed (this is the usual case of rotated log files); This option is meaningful only when polling (i.e., with --use-polling) and when --follow=name'
            cand -q 'Never output headers giving file names'
            cand --quiet 'Never output headers giving file names'
            cand --silent 'Never output headers giving file names'
            cand -v 'Always output headers giving file names'
            cand --verbose 'Always output headers giving file names'
            cand -z 'Line delimiter is NUL, not newline'
            cand --zero-terminated 'Line delimiter is NUL, not newline'
            cand --use-polling 'Disable ''inotify'' support and use polling instead'
            cand --retry 'Keep trying to open a file if it is inaccessible'
            cand -F 'Same as --follow=name --retry'
            cand --presume-input-pipe 'presume-input-pipe'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
