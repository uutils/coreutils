
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_join' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_join'
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
        'uu_join' {
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'also print unpairable lines from file FILENUM, where
FILENUM is 1 or 2, corresponding to FILE1 or FILE2')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'like -a FILENUM, but suppress joined output lines')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'replace missing input fields with EMPTY')
            [CompletionResult]::new('-j', 'j', [CompletionResultType]::ParameterName, 'equivalent to ''-1 FIELD -2 FIELD''')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'obey FORMAT while constructing output line')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'use CHAR as input and output field separator')
            [CompletionResult]::new('-1', '1', [CompletionResultType]::ParameterName, 'join on this FIELD of file 1')
            [CompletionResult]::new('-2', '2', [CompletionResultType]::ParameterName, 'join on this FIELD of file 2')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'ignore differences in case when comparing fields')
            [CompletionResult]::new('--ignore-case', 'ignore-case', [CompletionResultType]::ParameterName, 'ignore differences in case when comparing fields')
            [CompletionResult]::new('--check-order', 'check-order', [CompletionResultType]::ParameterName, 'check that the input is correctly sorted, even if all input lines are pairable')
            [CompletionResult]::new('--nocheck-order', 'nocheck-order', [CompletionResultType]::ParameterName, 'do not check that the input is correctly sorted')
            [CompletionResult]::new('--header', 'header', [CompletionResultType]::ParameterName, 'treat the first line in each file as field headers, print them without trying to pair them')
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
