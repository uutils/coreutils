
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_head' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_head'
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
        'uu_head' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'print the first NUM bytes of each file;
with the leading ''-'', print all but the last
NUM bytes of each file')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'print the first NUM bytes of each file;
with the leading ''-'', print all but the last
NUM bytes of each file')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'print the first NUM lines instead of the first 10;
with the leading ''-'', print all but the last
NUM lines of each file')
            [CompletionResult]::new('--lines', 'lines', [CompletionResultType]::ParameterName, 'print the first NUM lines instead of the first 10;
with the leading ''-'', print all but the last
NUM lines of each file')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'never print headers giving file names')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'never print headers giving file names')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'never print headers giving file names')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'always print headers giving file names')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'always print headers giving file names')
            [CompletionResult]::new('--presume-input-pipe', 'presume-input-pipe', [CompletionResultType]::ParameterName, 'presume-input-pipe')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
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
