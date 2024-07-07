
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_dircolors' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_dircolors'
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
        'uu_dircolors' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'output Bourne shell code to set LS_COLORS')
            [CompletionResult]::new('--sh', 'sh', [CompletionResultType]::ParameterName, 'output Bourne shell code to set LS_COLORS')
            [CompletionResult]::new('--bourne-shell', 'bourne-shell', [CompletionResultType]::ParameterName, 'output Bourne shell code to set LS_COLORS')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'output C shell code to set LS_COLORS')
            [CompletionResult]::new('--csh', 'csh', [CompletionResultType]::ParameterName, 'output C shell code to set LS_COLORS')
            [CompletionResult]::new('--c-shell', 'c-shell', [CompletionResultType]::ParameterName, 'output C shell code to set LS_COLORS')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'print the byte counts')
            [CompletionResult]::new('--print-database', 'print-database', [CompletionResultType]::ParameterName, 'print the byte counts')
            [CompletionResult]::new('--print-ls-colors', 'print-ls-colors', [CompletionResultType]::ParameterName, 'output fully escaped colors for display')
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
