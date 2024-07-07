
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_basename' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_basename'
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
        'uu_basename' {
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'remove a trailing SUFFIX; implies -a')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'remove a trailing SUFFIX; implies -a')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'support multiple arguments and treat each as a NAME')
            [CompletionResult]::new('--multiple', 'multiple', [CompletionResultType]::ParameterName, 'support multiple arguments and treat each as a NAME')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'end each output line with NUL, not newline')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'end each output line with NUL, not newline')
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
