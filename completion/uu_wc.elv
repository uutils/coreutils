
use builtin;
use str;

set edit:completion:arg-completer[uu_wc] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_wc'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_wc'= {
            cand --files0-from 'read input from the files specified by
  NUL-terminated names in file F;
  If F is - then read names from standard input'
            cand --total 'when to print a line with total counts;
  WHEN can be: auto, always, only, never'
            cand -c 'print the byte counts'
            cand --bytes 'print the byte counts'
            cand -m 'print the character counts'
            cand --chars 'print the character counts'
            cand -l 'print the newline counts'
            cand --lines 'print the newline counts'
            cand -L 'print the length of the longest line'
            cand --max-line-length 'print the length of the longest line'
            cand -w 'print the word counts'
            cand --words 'print the word counts'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
