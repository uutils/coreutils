
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_more' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_more'
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
        'uu_more' {
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'Display file beginning from pattern match')
            [CompletionResult]::new('--pattern', 'pattern', [CompletionResultType]::ParameterName, 'Display file beginning from pattern match')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'Display file beginning from line number')
            [CompletionResult]::new('--from-line', 'from-line', [CompletionResultType]::ParameterName, 'Display file beginning from line number')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'The number of lines per screen full')
            [CompletionResult]::new('--lines', 'lines', [CompletionResultType]::ParameterName, 'The number of lines per screen full')
            [CompletionResult]::new('--number', 'number', [CompletionResultType]::ParameterName, 'Same as --lines')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Do not scroll, display text and clean line ends')
            [CompletionResult]::new('--print-over', 'print-over', [CompletionResultType]::ParameterName, 'Do not scroll, display text and clean line ends')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Display help instead of ringing bell')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'Display help instead of ringing bell')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Do not scroll, clean screen and display text')
            [CompletionResult]::new('--clean-print', 'clean-print', [CompletionResultType]::ParameterName, 'Do not scroll, clean screen and display text')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Squeeze multiple blank lines into one')
            [CompletionResult]::new('--squeeze', 'squeeze', [CompletionResultType]::ParameterName, 'Squeeze multiple blank lines into one')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'u')
            [CompletionResult]::new('--plain', 'plain', [CompletionResultType]::ParameterName, 'plain')
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
