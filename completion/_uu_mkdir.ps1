
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_mkdir' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_mkdir'
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
        'uu_mkdir' {
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'set file mode (not implemented on windows)')
            [CompletionResult]::new('--mode', 'mode', [CompletionResultType]::ParameterName, 'set file mode (not implemented on windows)')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'make parent directories as needed')
            [CompletionResult]::new('--parents', 'parents', [CompletionResultType]::ParameterName, 'make parent directories as needed')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'print a message for each printed directory')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'print a message for each printed directory')
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
