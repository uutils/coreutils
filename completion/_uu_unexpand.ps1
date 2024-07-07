
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_unexpand' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_unexpand'
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
        'uu_unexpand' {
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)')
            [CompletionResult]::new('--tabs', 'tabs', [CompletionResultType]::ParameterName, 'use comma separated LIST of tab positions or have tabs N characters apart instead of 8 (enables -a)')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'convert all blanks, instead of just initial blanks')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'convert all blanks, instead of just initial blanks')
            [CompletionResult]::new('--first-only', 'first-only', [CompletionResultType]::ParameterName, 'convert only leading sequences of blanks (overrides -a)')
            [CompletionResult]::new('-U', 'U ', [CompletionResultType]::ParameterName, 'interpret input file as 8-bit ASCII rather than UTF-8')
            [CompletionResult]::new('--no-utf8', 'no-utf8', [CompletionResultType]::ParameterName, 'interpret input file as 8-bit ASCII rather than UTF-8')
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
