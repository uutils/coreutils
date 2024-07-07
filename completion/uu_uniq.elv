
use builtin;
use str;

set edit:completion:arg-completer[uu_uniq] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_uniq'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_uniq'= {
            cand -D 'print all duplicate lines. Delimiting is done with blank lines. [default: none]'
            cand --all-repeated 'print all duplicate lines. Delimiting is done with blank lines. [default: none]'
            cand --group 'show all items, separating groups with an empty line. [default: separate]'
            cand -w 'compare no more than N characters in lines'
            cand --check-chars 'compare no more than N characters in lines'
            cand -s 'avoid comparing the first N characters'
            cand --skip-chars 'avoid comparing the first N characters'
            cand -f 'avoid comparing the first N fields'
            cand --skip-fields 'avoid comparing the first N fields'
            cand -c 'prefix lines by the number of occurrences'
            cand --count 'prefix lines by the number of occurrences'
            cand -i 'ignore differences in case when comparing'
            cand --ignore-case 'ignore differences in case when comparing'
            cand -d 'only print duplicate lines'
            cand --repeated 'only print duplicate lines'
            cand -u 'only print unique lines'
            cand --unique 'only print unique lines'
            cand -z 'end lines with 0 byte, not newline'
            cand --zero-terminated 'end lines with 0 byte, not newline'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
