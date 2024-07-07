
use builtin;
use str;

set edit:completion:arg-completer[uu_touch] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_touch'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_touch'= {
            cand -t 'use [[CC]YY]MMDDhhmm[.ss] instead of the current time'
            cand -d 'parse argument and use it instead of current time'
            cand --date 'parse argument and use it instead of current time'
            cand -r 'use this file''s times instead of the current time'
            cand --reference 'use this file''s times instead of the current time'
            cand --time 'change only the specified time: "access", "atime", or "use" are equivalent to -a; "modify" or "mtime" are equivalent to -m'
            cand --help 'Print help information.'
            cand -a 'change only the access time'
            cand -m 'change only the modification time'
            cand -c 'do not create any files'
            cand --no-create 'do not create any files'
            cand -h 'affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)'
            cand --no-dereference 'affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
