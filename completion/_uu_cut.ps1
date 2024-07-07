
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_cut' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_cut'
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
        'uu_cut' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'filter byte columns from the input source')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'filter byte columns from the input source')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'alias for character mode')
            [CompletionResult]::new('--characters', 'characters', [CompletionResultType]::ParameterName, 'alias for character mode')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'specify the delimiter character that separates fields in the input source. Defaults to Tab.')
            [CompletionResult]::new('--delimiter', 'delimiter', [CompletionResultType]::ParameterName, 'specify the delimiter character that separates fields in the input source. Defaults to Tab.')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'filter field columns from the input source')
            [CompletionResult]::new('--fields', 'fields', [CompletionResultType]::ParameterName, 'filter field columns from the input source')
            [CompletionResult]::new('--output-delimiter', 'output-delimiter', [CompletionResultType]::ParameterName, 'in field mode, replace the delimiter in output lines with this option''s argument')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'Use any number of whitespace (Space, Tab) to separate fields in the input source (FreeBSD extension).')
            [CompletionResult]::new('--complement', 'complement', [CompletionResultType]::ParameterName, 'invert the filter - instead of displaying only the filtered columns, display all but those columns')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'in field mode, only print lines which contain the delimiter')
            [CompletionResult]::new('--only-delimited', 'only-delimited', [CompletionResultType]::ParameterName, 'in field mode, only print lines which contain the delimiter')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'instead of filtering columns based on line, filter columns based on \0 (NULL character)')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'instead of filtering columns based on line, filter columns based on \0 (NULL character)')
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
