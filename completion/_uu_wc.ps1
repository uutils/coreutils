
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_wc' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_wc'
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
        'uu_wc' {
            [CompletionResult]::new('--files0-from', 'files0-from', [CompletionResultType]::ParameterName, 'read input from the files specified by
  NUL-terminated names in file F;
  If F is - then read names from standard input')
            [CompletionResult]::new('--total', 'total', [CompletionResultType]::ParameterName, 'when to print a line with total counts;
  WHEN can be: auto, always, only, never')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'print the byte counts')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'print the byte counts')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'print the character counts')
            [CompletionResult]::new('--chars', 'chars', [CompletionResultType]::ParameterName, 'print the character counts')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'print the newline counts')
            [CompletionResult]::new('--lines', 'lines', [CompletionResultType]::ParameterName, 'print the newline counts')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'print the length of the longest line')
            [CompletionResult]::new('--max-line-length', 'max-line-length', [CompletionResultType]::ParameterName, 'print the length of the longest line')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'print the word counts')
            [CompletionResult]::new('--words', 'words', [CompletionResultType]::ParameterName, 'print the word counts')
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
