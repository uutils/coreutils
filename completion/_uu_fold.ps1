
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_fold' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_fold'
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
        'uu_fold' {
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'set WIDTH as the maximum line width rather than 80')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'set WIDTH as the maximum line width rather than 80')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'count using bytes rather than columns (meaning control characters such as newline are not treated specially)')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'count using bytes rather than columns (meaning control characters such as newline are not treated specially)')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'break lines at word boundaries rather than a hard cut-off')
            [CompletionResult]::new('--spaces', 'spaces', [CompletionResultType]::ParameterName, 'break lines at word boundaries rather than a hard cut-off')
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
