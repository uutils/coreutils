fmt-about = Reformat paragraphs from input (or standard input) to stdout.
fmt-usage = [OPTION]... [FILE]...

# Help messages
fmt-crown-margin-help = First and second line of paragraph may have different indentations, in which case the first line's indentation is preserved, and each subsequent line's indentation matches the second line.
fmt-tagged-paragraph-help = Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.
fmt-preserve-headers-help = Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.
fmt-split-only-help = Split lines only, do not reflow.
fmt-uniform-spacing-help = Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.
fmt-prefix-help = Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.
fmt-skip-prefix-help = Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP
fmt-exact-prefix-help = PREFIX must match at the beginning of the line with no preceding whitespace.
fmt-exact-skip-prefix-help = PSKIP must match at the beginning of the line with no preceding whitespace.
fmt-width-help = Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.
fmt-goal-help = Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.
fmt-quick-help = Break lines more quickly at the expense of a potentially more ragged appearance.
fmt-tab-width-help = Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.

# Error messages
fmt-error-invalid-goal = invalid goal: {$goal}
fmt-error-goal-greater-than-width = GOAL cannot be greater than WIDTH.
fmt-error-invalid-width = invalid width: {$width}
fmt-error-width-out-of-range = invalid width: '{$width}': Numerical result out of range
fmt-error-invalid-tabwidth = Invalid TABWIDTH specification: {$tabwidth}
fmt-error-first-option-width = invalid option -- {$option}; -WIDTH is recognized only when it is the first
  option; use -w N instead
  Try 'fmt --help' for more information.
fmt-error-read = read error
fmt-error-invalid-width-malformed = invalid width: {$width}
fmt-error-cannot-open-for-reading = cannot open {$file} for reading
fmt-error-cannot-get-metadata = cannot get metadata for {$file}
fmt-error-failed-to-write-output = failed to write output
