
use builtin;
use str;

set edit:completion:arg-completer[uu_csplit] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_csplit'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_csplit'= {
            cand -b 'use sprintf FORMAT instead of %02d'
            cand --suffix-format 'use sprintf FORMAT instead of %02d'
            cand -f 'use PREFIX instead of ''xx'''
            cand --prefix 'use PREFIX instead of ''xx'''
            cand -n 'use specified number of digits instead of 2'
            cand --digits 'use specified number of digits instead of 2'
            cand -k 'do not remove output files on errors'
            cand --keep-files 'do not remove output files on errors'
            cand --suppress-matched 'suppress the lines matching PATTERN'
            cand -s 'do not print counts of output file sizes'
            cand --quiet 'do not print counts of output file sizes'
            cand --silent 'do not print counts of output file sizes'
            cand -z 'remove empty output files'
            cand --elide-empty-files 'remove empty output files'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
