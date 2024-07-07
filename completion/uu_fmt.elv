
use builtin;
use str;

set edit:completion:arg-completer[uu_fmt] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_fmt'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_fmt'= {
            cand -p 'Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.'
            cand --prefix 'Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.'
            cand -P 'Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP'
            cand --skip-prefix 'Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP'
            cand -w 'Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.'
            cand --width 'Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.'
            cand -g 'Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.'
            cand --goal 'Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.'
            cand -T 'Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.'
            cand --tab-width 'Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.'
            cand -c 'First and second line of paragraph may have different indentations, in which case the first line''s indentation is preserved, and each subsequent line''s indentation matches the second line.'
            cand --crown-margin 'First and second line of paragraph may have different indentations, in which case the first line''s indentation is preserved, and each subsequent line''s indentation matches the second line.'
            cand -t 'Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.'
            cand --tagged-paragraph 'Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.'
            cand -m 'Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.'
            cand --preserve-headers 'Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.'
            cand -s 'Split lines only, do not reflow.'
            cand --split-only 'Split lines only, do not reflow.'
            cand -u 'Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.'
            cand --uniform-spacing 'Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.'
            cand -x 'PREFIX must match at the beginning of the line with no preceding whitespace.'
            cand --exact-prefix 'PREFIX must match at the beginning of the line with no preceding whitespace.'
            cand -X 'PSKIP must match at the beginning of the line with no preceding whitespace.'
            cand --exact-skip-prefix 'PSKIP must match at the beginning of the line with no preceding whitespace.'
            cand -q 'Break lines more quickly at the expense of a potentially more ragged appearance.'
            cand --quick 'Break lines more quickly at the expense of a potentially more ragged appearance.'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
