
use builtin;
use str;

set edit:completion:arg-completer[uu_pr] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_pr'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_pr'= {
            cand --pages 'Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]'
            cand -h 'Use the string header to replace the file name in the header line.'
            cand --header 'Use the string header to replace the file name in the header line.'
            cand -n 'Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies the first width column positions of each text column or each line of -m output.  If char (any non-digit character) is given, it is appended to the line number to separate it from whatever follows.  The default for char is a <tab>. Line numbers longer than width columns are truncated.'
            cand --number-lines 'Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies the first width column positions of each text column or each line of -m output.  If char (any non-digit character) is given, it is appended to the line number to separate it from whatever follows.  The default for char is a <tab>. Line numbers longer than width columns are truncated.'
            cand -N 'start counting with NUMBER at 1st line of first page printed'
            cand --first-line-number 'start counting with NUMBER at 1st line of first page printed'
            cand -l 'Override the 66-line default (default number of lines of text 56, and with -F 63) and reset the page length to lines.  If lines is not greater than the sum  of  both the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the -t option were in effect. '
            cand --length 'Override the 66-line default (default number of lines of text 56, and with -F 63) and reset the page length to lines.  If lines is not greater than the sum  of  both the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the -t option were in effect. '
            cand -w 'Set the width of the line to width column positions for multiple text-column output only. If the -w option is not specified and the -s option is not specified, the default width shall be 72. If the -w option is not specified and the -s option is specified, the default width shall be 512.'
            cand --width 'Set the width of the line to width column positions for multiple text-column output only. If the -w option is not specified and the -s option is not specified, the default width shall be 72. If the -w option is not specified and the -s option is specified, the default width shall be 512.'
            cand -W 'set page width to PAGE_WIDTH (72) characters always, truncate lines, except -J option is set, no interference with -S or -s'
            cand --page-width 'set page width to PAGE_WIDTH (72) characters always, truncate lines, except -J option is set, no interference with -S or -s'
            cand --column 'Produce multi-column output that is arranged in column columns (the default shall be 1) and is written down each column  in  the order in which the text is received from the input file. This option should not be used with -m. The options -e and -i shall be assumed for multiple text-column output.  Whether or not text columns are produced with identical vertical lengths is unspecified, but a text column shall never exceed the length of the page (see the -l option). When used with -t, use the minimum number of lines to write the output.'
            cand -s 'Separate text columns by the single character char instead of by the appropriate number of <space>s (default for char is the <tab> character).'
            cand --separator 'Separate text columns by the single character char instead of by the appropriate number of <space>s (default for char is the <tab> character).'
            cand -S 'separate columns by STRING, without -S: Default separator <TAB> with -J and <space> otherwise (same as -S" "), no effect on column options'
            cand --sep-string 'separate columns by STRING, without -S: Default separator <TAB> with -J and <space> otherwise (same as -S" "), no effect on column options'
            cand -o 'Each line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset shall be zero. The space taken is in addition to the output line width (see the -w option below).'
            cand --indent 'Each line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset shall be zero. The space taken is in addition to the output line width (see the -w option below).'
            cand -d 'Produce output that is double spaced. An extra <newline> character is output following every <newline> found in the input.'
            cand --double-space 'Produce output that is double spaced. An extra <newline> character is output following every <newline> found in the input.'
            cand -t 'Write neither the five-line identifying header nor the five-line trailer usually supplied for each page. Quit writing after the last line of each file without spacing to the end of the page.'
            cand --omit-header 'Write neither the five-line identifying header nor the five-line trailer usually supplied for each page. Quit writing after the last line of each file without spacing to the end of the page.'
            cand -r 'omit warning when a file cannot be opened'
            cand --no-file-warnings 'omit warning when a file cannot be opened'
            cand -F 'Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.'
            cand --form-feed 'Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.'
            cand -a 'Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the second line in column 1, and so on).'
            cand --across 'Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the second line in column 1, and so on).'
            cand -m 'Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by a file operand, side by side into text columns of equal fixed widths, in terms of the number of column positions. Implementations shall support merging of at least nine file operands.'
            cand --merge 'Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by a file operand, side by side into text columns of equal fixed widths, in terms of the number of column positions. Implementations shall support merging of at least nine file operands.'
            cand -J 'merge full lines, turns off -W line truncation, no column alignment, --sep-string[=STRING] sets separators'
            cand --help 'Print help information'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
