
use builtin;
use str;

set edit:completion:arg-completer[uu_ptx] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_ptx'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_ptx'= {
            cand -F 'use STRING for flagging line truncations'
            cand --flag-truncation 'use STRING for flagging line truncations'
            cand -M 'macro name to use instead of ''xx'''
            cand --macro-name 'macro name to use instead of ''xx'''
            cand -S 'for end of lines or end of sentences'
            cand --sentence-regexp 'for end of lines or end of sentences'
            cand -W 'use REGEXP to match each keyword'
            cand --word-regexp 'use REGEXP to match each keyword'
            cand -b 'word break characters in this FILE'
            cand --break-file 'word break characters in this FILE'
            cand -g 'gap size in columns between output fields'
            cand --gap-size 'gap size in columns between output fields'
            cand -i 'read ignore word list from FILE'
            cand --ignore-file 'read ignore word list from FILE'
            cand -o 'read only word list from this FILE'
            cand --only-file 'read only word list from this FILE'
            cand -w 'output width in columns, reference excluded'
            cand --width 'output width in columns, reference excluded'
            cand -A 'output automatically generated references'
            cand --auto-reference 'output automatically generated references'
            cand -G 'behave more like System V ''ptx'''
            cand --traditional 'behave more like System V ''ptx'''
            cand -O 'generate output as roff directives'
            cand --format=roff 'generate output as roff directives'
            cand -R 'put references at right, not counted in -w'
            cand --right-side-refs 'put references at right, not counted in -w'
            cand -T 'generate output as TeX directives'
            cand --format=tex 'generate output as TeX directives'
            cand -f 'fold lower case to upper case for sorting'
            cand --ignore-case 'fold lower case to upper case for sorting'
            cand -r 'first field of each line is a reference'
            cand --references 'first field of each line is a reference'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
