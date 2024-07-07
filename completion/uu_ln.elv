
use builtin;
use str;

set edit:completion:arg-completer[uu_ln] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_ln'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_ln'= {
            cand --backup 'make a backup of each existing destination file'
            cand -S 'override the usual backup suffix'
            cand --suffix 'override the usual backup suffix'
            cand -t 'specify the DIRECTORY in which to create the links'
            cand --target-directory 'specify the DIRECTORY in which to create the links'
            cand -b 'like --backup but does not accept an argument'
            cand -f 'remove existing destination files'
            cand --force 'remove existing destination files'
            cand -i 'prompt whether to remove existing destination files'
            cand --interactive 'prompt whether to remove existing destination files'
            cand -n 'treat LINK_NAME as a normal file if it is a symbolic link to a directory'
            cand --no-dereference 'treat LINK_NAME as a normal file if it is a symbolic link to a directory'
            cand -L 'follow TARGETs that are symbolic links'
            cand --logical 'follow TARGETs that are symbolic links'
            cand -P 'make hard links directly to symbolic links'
            cand --physical 'make hard links directly to symbolic links'
            cand -s 'make symbolic links instead of hard links'
            cand --symbolic 'make symbolic links instead of hard links'
            cand -T 'treat LINK_NAME as a normal file always'
            cand --no-target-directory 'treat LINK_NAME as a normal file always'
            cand -r 'create symbolic links relative to link location'
            cand --relative 'create symbolic links relative to link location'
            cand -v 'print name of each linked file'
            cand --verbose 'print name of each linked file'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
