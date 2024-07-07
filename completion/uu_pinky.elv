
use builtin;
use str;

set edit:completion:arg-completer[uu_pinky] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_pinky'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_pinky'= {
            cand -l 'produce long format output for the specified USERs'
            cand -b 'omit the user''s home directory and shell in long format'
            cand -h 'omit the user''s project file in long format'
            cand -p 'omit the user''s plan file in long format'
            cand -s 'do short format output, this is the default'
            cand -f 'omit the line of column headings in short format'
            cand -w 'omit the user''s full name in short format'
            cand -i 'omit the user''s full name and remote host in short format'
            cand -q 'omit the user''s full name, remote host and idle time in short format'
            cand --help 'Print help information'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
