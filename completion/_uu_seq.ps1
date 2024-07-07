
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_seq' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_seq'
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
        'uu_seq' {
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Separator character (defaults to \n)')
            [CompletionResult]::new('--separator', 'separator', [CompletionResultType]::ParameterName, 'Separator character (defaults to \n)')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Terminator character (defaults to \n)')
            [CompletionResult]::new('--terminator', 'terminator', [CompletionResultType]::ParameterName, 'Terminator character (defaults to \n)')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'use printf style floating-point FORMAT')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'use printf style floating-point FORMAT')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'Equalize widths of all numbers by padding with zeros')
            [CompletionResult]::new('--equal-width', 'equal-width', [CompletionResultType]::ParameterName, 'Equalize widths of all numbers by padding with zeros')
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
