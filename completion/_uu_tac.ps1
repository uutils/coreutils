
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_tac' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_tac'
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
        'uu_tac' {
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'use STRING as the separator instead of newline')
            [CompletionResult]::new('--separator', 'separator', [CompletionResultType]::ParameterName, 'use STRING as the separator instead of newline')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'attach the separator before instead of after')
            [CompletionResult]::new('--before', 'before', [CompletionResultType]::ParameterName, 'attach the separator before instead of after')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'interpret the sequence as a regular expression')
            [CompletionResult]::new('--regex', 'regex', [CompletionResultType]::ParameterName, 'interpret the sequence as a regular expression')
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
