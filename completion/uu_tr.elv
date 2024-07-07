
use builtin;
use str;

set edit:completion:arg-completer[uu_tr] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_tr'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_tr'= {
            cand -c 'use the complement of SET1'
            cand -C 'use the complement of SET1'
            cand --complement 'use the complement of SET1'
            cand -d 'delete characters in SET1, do not translate'
            cand --delete 'delete characters in SET1, do not translate'
            cand -s 'replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character'
            cand --squeeze-repeats 'replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character'
            cand -t 'first truncate SET1 to length of SET2'
            cand --truncate-set1 'first truncate SET1 to length of SET2'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
