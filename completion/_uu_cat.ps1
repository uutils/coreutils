
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_cat' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_cat'
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
        'uu_cat' {
            [CompletionResult]::new('-A', 'A ', [CompletionResultType]::ParameterName, 'equivalent to -vET')
            [CompletionResult]::new('--show-all', 'show-all', [CompletionResultType]::ParameterName, 'equivalent to -vET')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'number nonempty output lines, overrides -n')
            [CompletionResult]::new('--number-nonblank', 'number-nonblank', [CompletionResultType]::ParameterName, 'number nonempty output lines, overrides -n')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'equivalent to -vE')
            [CompletionResult]::new('-E', 'E ', [CompletionResultType]::ParameterName, 'display $ at end of each line')
            [CompletionResult]::new('--show-ends', 'show-ends', [CompletionResultType]::ParameterName, 'display $ at end of each line')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'number all output lines')
            [CompletionResult]::new('--number', 'number', [CompletionResultType]::ParameterName, 'number all output lines')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'suppress repeated empty output lines')
            [CompletionResult]::new('--squeeze-blank', 'squeeze-blank', [CompletionResultType]::ParameterName, 'suppress repeated empty output lines')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'equivalent to -vT')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'display TAB characters at ^I')
            [CompletionResult]::new('--show-tabs', 'show-tabs', [CompletionResultType]::ParameterName, 'display TAB characters at ^I')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'use ^ and M- notation, except for LF (\n) and TAB (\t)')
            [CompletionResult]::new('--show-nonprinting', 'show-nonprinting', [CompletionResultType]::ParameterName, 'use ^ and M- notation, except for LF (\n) and TAB (\t)')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, '(ignored)')
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
