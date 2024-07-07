
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_comm' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_comm'
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
        'uu_comm' {
            [CompletionResult]::new('--output-delimiter', 'output-delimiter', [CompletionResultType]::ParameterName, 'separate columns with STR')
            [CompletionResult]::new('-1', '1', [CompletionResultType]::ParameterName, 'suppress column 1 (lines unique to FILE1)')
            [CompletionResult]::new('-2', '2', [CompletionResultType]::ParameterName, 'suppress column 2 (lines unique to FILE2)')
            [CompletionResult]::new('-3', '3', [CompletionResultType]::ParameterName, 'suppress column 3 (lines that appear in both files)')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
            [CompletionResult]::new('--total', 'total', [CompletionResultType]::ParameterName, 'output a summary')
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
