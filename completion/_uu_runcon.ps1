
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_runcon' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_runcon'
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
        'uu_runcon' {
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Set user USER in the target security context.')
            [CompletionResult]::new('--user', 'user', [CompletionResultType]::ParameterName, 'Set user USER in the target security context.')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Set role ROLE in the target security context.')
            [CompletionResult]::new('--role', 'role', [CompletionResultType]::ParameterName, 'Set role ROLE in the target security context.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Set type TYPE in the target security context.')
            [CompletionResult]::new('--type', 'type', [CompletionResultType]::ParameterName, 'Set type TYPE in the target security context.')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Set range RANGE in the target security context.')
            [CompletionResult]::new('--range', 'range', [CompletionResultType]::ParameterName, 'Set range RANGE in the target security context.')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Compute process transition context before modifying.')
            [CompletionResult]::new('--compute', 'compute', [CompletionResultType]::ParameterName, 'Compute process transition context before modifying.')
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
