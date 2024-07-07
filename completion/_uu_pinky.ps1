
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_pinky' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_pinky'
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
        'uu_pinky' {
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'produce long format output for the specified USERs')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'omit the user''s home directory and shell in long format')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'omit the user''s project file in long format')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'omit the user''s plan file in long format')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'do short format output, this is the default')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'omit the line of column headings in short format')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'omit the user''s full name in short format')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'omit the user''s full name and remote host in short format')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'omit the user''s full name, remote host and idle time in short format')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
