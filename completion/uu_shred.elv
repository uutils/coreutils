
use builtin;
use str;

set edit:completion:arg-completer[uu_shred] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_shred'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_shred'= {
            cand -n 'overwrite N times instead of the default (3)'
            cand --iterations 'overwrite N times instead of the default (3)'
            cand -s 'shred this many bytes (suffixes like K, M, G accepted)'
            cand --size 'shred this many bytes (suffixes like K, M, G accepted)'
            cand --remove 'like -u but give control on HOW to delete;  See below'
            cand -f 'change permissions to allow writing if necessary'
            cand --force 'change permissions to allow writing if necessary'
            cand -u 'deallocate and remove file after overwriting'
            cand -v 'show progress'
            cand --verbose 'show progress'
            cand -x 'do not round file sizes up to the next full block;
this is the default for non-regular files'
            cand --exact 'do not round file sizes up to the next full block;
this is the default for non-regular files'
            cand -z 'add a final overwrite with zeros to hide shredding'
            cand --zero 'add a final overwrite with zeros to hide shredding'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
