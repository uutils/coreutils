
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_expand' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_expand'
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
        'uu_expand' {
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'have tabs N characters apart, not 8 or use comma separated list of explicit tab positions')
            [CompletionResult]::new('--tabs', 'tabs', [CompletionResultType]::ParameterName, 'have tabs N characters apart, not 8 or use comma separated list of explicit tab positions')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'do not convert tabs after non blanks')
            [CompletionResult]::new('--initial', 'initial', [CompletionResultType]::ParameterName, 'do not convert tabs after non blanks')
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
