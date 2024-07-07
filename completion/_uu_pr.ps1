
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_pr' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_pr'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'uu_pr' {
            [CompletionResult]::new('--pages', 'pages', [CompletionResultType]::ParameterName, 'Begin and stop printing with page FIRST_PAGE[:LAST_PAGE]')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Use the string header to replace the file name in the header line.')
            [CompletionResult]::new('--header', 'header', [CompletionResultType]::ParameterName, 'Use the string header to replace the file name in the header line.')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies the first width column positions of each text column or each line of -m output.  If char (any non-digit character) is given, it is appended to the line number to separate it from whatever follows.  The default for char is a <tab>. Line numbers longer than width columns are truncated.')
            [CompletionResult]::new('--number-lines', 'number-lines', [CompletionResultType]::ParameterName, 'Provide width digit line numbering.  The default for width, if not specified, is 5.  The number occupies the first width column positions of each text column or each line of -m output.  If char (any non-digit character) is given, it is appended to the line number to separate it from whatever follows.  The default for char is a <tab>. Line numbers longer than width columns are truncated.')
            [CompletionResult]::new('-N', 'N ', [CompletionResultType]::ParameterName, 'start counting with NUMBER at 1st line of first page printed')
            [CompletionResult]::new('--first-line-number', 'first-line-number', [CompletionResultType]::ParameterName, 'start counting with NUMBER at 1st line of first page printed')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Override the 66-line default (default number of lines of text 56, and with -F 63) and reset the page length to lines.  If lines is not greater than the sum  of  both the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the -t option were in effect. ')
            [CompletionResult]::new('--length', 'length', [CompletionResultType]::ParameterName, 'Override the 66-line default (default number of lines of text 56, and with -F 63) and reset the page length to lines.  If lines is not greater than the sum  of  both the  header  and trailer depths (in lines), the pr utility shall suppress both the header and trailer, as if the -t option were in effect. ')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'Set the width of the line to width column positions for multiple text-column output only. If the -w option is not specified and the -s option is not specified, the default width shall be 72. If the -w option is not specified and the -s option is specified, the default width shall be 512.')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'Set the width of the line to width column positions for multiple text-column output only. If the -w option is not specified and the -s option is not specified, the default width shall be 72. If the -w option is not specified and the -s option is specified, the default width shall be 512.')
            [CompletionResult]::new('-W', 'W ', [CompletionResultType]::ParameterName, 'set page width to PAGE_WIDTH (72) characters always, truncate lines, except -J option is set, no interference with -S or -s')
            [CompletionResult]::new('--page-width', 'page-width', [CompletionResultType]::ParameterName, 'set page width to PAGE_WIDTH (72) characters always, truncate lines, except -J option is set, no interference with -S or -s')
            [CompletionResult]::new('--column', 'column', [CompletionResultType]::ParameterName, 'Produce multi-column output that is arranged in column columns (the default shall be 1) and is written down each column  in  the order in which the text is received from the input file. This option should not be used with -m. The options -e and -i shall be assumed for multiple text-column output.  Whether or not text columns are produced with identical vertical lengths is unspecified, but a text column shall never exceed the length of the page (see the -l option). When used with -t, use the minimum number of lines to write the output.')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Separate text columns by the single character char instead of by the appropriate number of <space>s (default for char is the <tab> character).')
            [CompletionResult]::new('--separator', 'separator', [CompletionResultType]::ParameterName, 'Separate text columns by the single character char instead of by the appropriate number of <space>s (default for char is the <tab> character).')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'separate columns by STRING, without -S: Default separator <TAB> with -J and <space> otherwise (same as -S" "), no effect on column options')
            [CompletionResult]::new('--sep-string', 'sep-string', [CompletionResultType]::ParameterName, 'separate columns by STRING, without -S: Default separator <TAB> with -J and <space> otherwise (same as -S" "), no effect on column options')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'Each line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset shall be zero. The space taken is in addition to the output line width (see the -w option below).')
            [CompletionResult]::new('--indent', 'indent', [CompletionResultType]::ParameterName, 'Each line of output shall be preceded by offset <space>s. If the -o option is not specified, the default offset shall be zero. The space taken is in addition to the output line width (see the -w option below).')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Produce output that is double spaced. An extra <newline> character is output following every <newline> found in the input.')
            [CompletionResult]::new('--double-space', 'double-space', [CompletionResultType]::ParameterName, 'Produce output that is double spaced. An extra <newline> character is output following every <newline> found in the input.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Write neither the five-line identifying header nor the five-line trailer usually supplied for each page. Quit writing after the last line of each file without spacing to the end of the page.')
            [CompletionResult]::new('--omit-header', 'omit-header', [CompletionResultType]::ParameterName, 'Write neither the five-line identifying header nor the five-line trailer usually supplied for each page. Quit writing after the last line of each file without spacing to the end of the page.')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'omit warning when a file cannot be opened')
            [CompletionResult]::new('--no-file-warnings', 'no-file-warnings', [CompletionResultType]::ParameterName, 'omit warning when a file cannot be opened')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.')
            [CompletionResult]::new('--form-feed', 'form-feed', [CompletionResultType]::ParameterName, 'Use a <form-feed> for new pages, instead of the default behavior that uses a sequence of <newline>s.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the second line in column 1, and so on).')
            [CompletionResult]::new('--across', 'across', [CompletionResultType]::ParameterName, 'Modify the effect of the - column option so that the columns are filled across the page in a  round-robin  order (for example, when column is 2, the first input line heads column 1, the second heads column 2, the third is the second line in column 1, and so on).')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by a file operand, side by side into text columns of equal fixed widths, in terms of the number of column positions. Implementations shall support merging of at least nine file operands.')
            [CompletionResult]::new('--merge', 'merge', [CompletionResultType]::ParameterName, 'Merge files. Standard output shall be formatted so the pr utility writes one line from each file specified by a file operand, side by side into text columns of equal fixed widths, in terms of the number of column positions. Implementations shall support merging of at least nine file operands.')
            [CompletionResult]::new('-J', 'J ', [CompletionResultType]::ParameterName, 'merge full lines, turns off -W line truncation, no column alignment, --sep-string[=STRING] sets separators')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
