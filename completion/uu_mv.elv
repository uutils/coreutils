
use builtin;
use str;

set edit:completion:arg-completer[uu_mv] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_mv'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_mv'= {
            cand --backup 'make a backup of each existing destination file'
            cand -S 'override the usual backup suffix'
            cand --suffix 'override the usual backup suffix'
            cand --update 'move only when the SOURCE file is newer than the destination file or when the destination file is missing'
            cand -t 'move all SOURCE arguments into DIRECTORY'
            cand --target-directory 'move all SOURCE arguments into DIRECTORY'
            cand -f 'do not prompt before overwriting'
            cand --force 'do not prompt before overwriting'
            cand -i 'prompt before override'
            cand --interactive 'prompt before override'
            cand -n 'do not overwrite an existing file'
            cand --no-clobber 'do not overwrite an existing file'
            cand --strip-trailing-slashes 'remove any trailing slashes from each SOURCE argument'
            cand -b 'like --backup but does not accept an argument'
            cand -u 'like --update but does not accept an argument'
            cand -T 'treat DEST as a normal file'
            cand --no-target-directory 'treat DEST as a normal file'
            cand -v 'explain what is being done'
            cand --verbose 'explain what is being done'
            cand -g 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
            cand --progress 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
