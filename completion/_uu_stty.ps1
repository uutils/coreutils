
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_stty' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_stty'
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
        'uu_stty' {
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'open and use the specified DEVICE instead of stdin')
            [CompletionResult]::new('--file', 'file', [CompletionResultType]::ParameterName, 'open and use the specified DEVICE instead of stdin')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'print all current settings in human-readable form')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'print all current settings in human-readable form')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'print all current settings in a stty-readable form')
            [CompletionResult]::new('--save', 'save', [CompletionResultType]::ParameterName, 'print all current settings in a stty-readable form')
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
