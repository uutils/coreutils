
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_fmt' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_fmt'
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
        'uu_fmt' {
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.')
            [CompletionResult]::new('--prefix', 'prefix', [CompletionResultType]::ParameterName, 'Reformat only lines beginning with PREFIX, reattaching PREFIX to reformatted lines. Unless -x is specified, leading whitespace will be ignored when matching PREFIX.')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP')
            [CompletionResult]::new('--skip-prefix', 'skip-prefix', [CompletionResultType]::ParameterName, 'Do not reformat lines beginning with PSKIP. Unless -X is specified, leading whitespace will be ignored when matching PSKIP')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'Fill output lines up to a maximum of WIDTH columns, default 75. This can be specified as a negative number in the first argument.')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.')
            [CompletionResult]::new('--goal', 'goal', [CompletionResultType]::ParameterName, 'Goal width, default of 93% of WIDTH. Must be less than or equal to WIDTH.')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.')
            [CompletionResult]::new('--tab-width', 'tab-width', [CompletionResultType]::ParameterName, 'Treat tabs as TABWIDTH spaces for determining line length, default 8. Note that this is used only for calculating line lengths; tabs are preserved in the output.')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'First and second line of paragraph may have different indentations, in which case the first line''s indentation is preserved, and each subsequent line''s indentation matches the second line.')
            [CompletionResult]::new('--crown-margin', 'crown-margin', [CompletionResultType]::ParameterName, 'First and second line of paragraph may have different indentations, in which case the first line''s indentation is preserved, and each subsequent line''s indentation matches the second line.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.')
            [CompletionResult]::new('--tagged-paragraph', 'tagged-paragraph', [CompletionResultType]::ParameterName, 'Like -c, except that the first and second line of a paragraph *must* have different indentation or they are treated as separate paragraphs.')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.')
            [CompletionResult]::new('--preserve-headers', 'preserve-headers', [CompletionResultType]::ParameterName, 'Attempt to detect and preserve mail headers in the input. Be careful when combining this flag with -p.')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Split lines only, do not reflow.')
            [CompletionResult]::new('--split-only', 'split-only', [CompletionResultType]::ParameterName, 'Split lines only, do not reflow.')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.')
            [CompletionResult]::new('--uniform-spacing', 'uniform-spacing', [CompletionResultType]::ParameterName, 'Insert exactly one space between words, and two between sentences. Sentence breaks in the input are detected as [?!.] followed by two spaces or a newline; other punctuation is not interpreted as a sentence break.')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'PREFIX must match at the beginning of the line with no preceding whitespace.')
            [CompletionResult]::new('--exact-prefix', 'exact-prefix', [CompletionResultType]::ParameterName, 'PREFIX must match at the beginning of the line with no preceding whitespace.')
            [CompletionResult]::new('-X', 'X ', [CompletionResultType]::ParameterName, 'PSKIP must match at the beginning of the line with no preceding whitespace.')
            [CompletionResult]::new('--exact-skip-prefix', 'exact-skip-prefix', [CompletionResultType]::ParameterName, 'PSKIP must match at the beginning of the line with no preceding whitespace.')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Break lines more quickly at the expense of a potentially more ragged appearance.')
            [CompletionResult]::new('--quick', 'quick', [CompletionResultType]::ParameterName, 'Break lines more quickly at the expense of a potentially more ragged appearance.')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
