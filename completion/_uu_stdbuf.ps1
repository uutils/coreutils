
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_stdbuf' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_stdbuf'
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
        'uu_stdbuf' {
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'adjust standard input stream buffering')
            [CompletionResult]::new('--input', 'input', [CompletionResultType]::ParameterName, 'adjust standard input stream buffering')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'adjust standard output stream buffering')
            [CompletionResult]::new('--output', 'output', [CompletionResultType]::ParameterName, 'adjust standard output stream buffering')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'adjust standard error stream buffering')
            [CompletionResult]::new('--error', 'error', [CompletionResultType]::ParameterName, 'adjust standard error stream buffering')
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
