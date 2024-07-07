
use builtin;
use str;

set edit:completion:arg-completer[uu_cut] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_cut'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_cut'= {
            cand -b 'filter byte columns from the input source'
            cand --bytes 'filter byte columns from the input source'
            cand -c 'alias for character mode'
            cand --characters 'alias for character mode'
            cand -d 'specify the delimiter character that separates fields in the input source. Defaults to Tab.'
            cand --delimiter 'specify the delimiter character that separates fields in the input source. Defaults to Tab.'
            cand -f 'filter field columns from the input source'
            cand --fields 'filter field columns from the input source'
            cand --output-delimiter 'in field mode, replace the delimiter in output lines with this option''s argument'
            cand -w 'Use any number of whitespace (Space, Tab) to separate fields in the input source (FreeBSD extension).'
            cand --complement 'invert the filter - instead of displaying only the filtered columns, display all but those columns'
            cand -s 'in field mode, only print lines which contain the delimiter'
            cand --only-delimited 'in field mode, only print lines which contain the delimiter'
            cand -z 'instead of filtering columns based on line, filter columns based on \0 (NULL character)'
            cand --zero-terminated 'instead of filtering columns based on line, filter columns based on \0 (NULL character)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
