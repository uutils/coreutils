
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_shuf' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_shuf'
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
        'uu_shuf' {
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'treat each number LO through HI as an input line')
            [CompletionResult]::new('--input-range', 'input-range', [CompletionResultType]::ParameterName, 'treat each number LO through HI as an input line')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'output at most COUNT lines')
            [CompletionResult]::new('--head-count', 'head-count', [CompletionResultType]::ParameterName, 'output at most COUNT lines')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'write result to FILE instead of standard output')
            [CompletionResult]::new('--output', 'output', [CompletionResultType]::ParameterName, 'write result to FILE instead of standard output')
            [CompletionResult]::new('--random-source', 'random-source', [CompletionResultType]::ParameterName, 'get random bytes from FILE')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'treat each ARG as an input line')
            [CompletionResult]::new('--echo', 'echo', [CompletionResultType]::ParameterName, 'treat each ARG as an input line')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'output lines can be repeated')
            [CompletionResult]::new('--repeat', 'repeat', [CompletionResultType]::ParameterName, 'output lines can be repeated')
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
