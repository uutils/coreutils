
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_tee' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_tee'
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
        'uu_tee' {
            [CompletionResult]::new('--output-error', 'output-error', [CompletionResultType]::ParameterName, 'set write error behavior')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'append to the given FILEs, do not overwrite')
            [CompletionResult]::new('--append', 'append', [CompletionResultType]::ParameterName, 'append to the given FILEs, do not overwrite')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'ignore interrupt signals (ignored on non-Unix platforms)')
            [CompletionResult]::new('--ignore-interrupts', 'ignore-interrupts', [CompletionResultType]::ParameterName, 'ignore interrupt signals (ignored on non-Unix platforms)')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'set write error behavior (ignored on non-Unix platforms)')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
