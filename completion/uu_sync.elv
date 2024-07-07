
use builtin;
use str;

set edit:completion:arg-completer[uu_sync] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_sync'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_sync'= {
            cand -f 'sync the file systems that contain the files (Linux and Windows only)'
            cand --file-system 'sync the file systems that contain the files (Linux and Windows only)'
            cand -d 'sync only file data, no unneeded metadata (Linux only)'
            cand --data 'sync only file data, no unneeded metadata (Linux only)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
