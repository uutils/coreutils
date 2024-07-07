
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_kill' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_kill'
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
        'uu_kill' {
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Sends given signal instead of SIGTERM')
            [CompletionResult]::new('--signal', 'signal', [CompletionResultType]::ParameterName, 'Sends given signal instead of SIGTERM')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Lists signals')
            [CompletionResult]::new('--list', 'list', [CompletionResultType]::ParameterName, 'Lists signals')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Lists table of signals')
            [CompletionResult]::new('--table', 'table', [CompletionResultType]::ParameterName, 'Lists table of signals')
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
