
use builtin;
use str;

set edit:completion:arg-completer[uu_more] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_more'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_more'= {
            cand -P 'Display file beginning from pattern match'
            cand --pattern 'Display file beginning from pattern match'
            cand -F 'Display file beginning from line number'
            cand --from-line 'Display file beginning from line number'
            cand -n 'The number of lines per screen full'
            cand --lines 'The number of lines per screen full'
            cand --number 'Same as --lines'
            cand -c 'Do not scroll, display text and clean line ends'
            cand --print-over 'Do not scroll, display text and clean line ends'
            cand -d 'Display help instead of ringing bell'
            cand --silent 'Display help instead of ringing bell'
            cand -p 'Do not scroll, clean screen and display text'
            cand --clean-print 'Do not scroll, clean screen and display text'
            cand -s 'Squeeze multiple blank lines into one'
            cand --squeeze 'Squeeze multiple blank lines into one'
            cand -u 'u'
            cand --plain 'plain'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
