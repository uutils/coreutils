complete -c uu_fmt -s p -l prefix -d 'Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.' -r
complete -c uu_fmt -s P -l skip-prefix -d 'Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP' -r
complete -c uu_fmt -s w -l width -d 'Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.' -r
complete -c uu_fmt -s g -l goal -d 'Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.' -r
complete -c uu_fmt -s T -l tab-width -d 'Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.' -r
complete -c uu_fmt -s c -l crown-margin -d 'First and second line of paragraph may have different indentations, in which case the first line\'s indentation is preserved, and each subsequent line\'s indentation matches the second line.'
complete -c uu_fmt -s t -l tagged-paragraph -d 'Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.'
complete -c uu_fmt -s m -l preserve-headers -d 'Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.'
complete -c uu_fmt -s s -l split-only -d 'Split lines only, do not reflow.'
complete -c uu_fmt -s u -l uniform-spacing -d 'Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.'
complete -c uu_fmt -s x -l exact-prefix -d 'PREFIX must match at the beginning of the line with no preceding whitespace.'
complete -c uu_fmt -s X -l exact-skip-prefix -d 'PSKIP must match at the beginning of the line with no preceding whitespace.'
complete -c uu_fmt -s q -l quick -d 'Break lines more quickly at the expense of a potentially more ragged appearance.'
complete -c uu_fmt -s h -l help -d 'Print help'
complete -c uu_fmt -s V -l version -d 'Print version'
