
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_uniq' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_uniq'
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
        'uu_uniq' {
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'print all duplicate lines. Delimiting is done with blank lines. [default: none]')
            [CompletionResult]::new('--all-repeated', 'all-repeated', [CompletionResultType]::ParameterName, 'print all duplicate lines. Delimiting is done with blank lines. [default: none]')
            [CompletionResult]::new('--group', 'group', [CompletionResultType]::ParameterName, 'show all items, separating groups with an empty line. [default: separate]')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'compare no more than N characters in lines')
            [CompletionResult]::new('--check-chars', 'check-chars', [CompletionResultType]::ParameterName, 'compare no more than N characters in lines')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'avoid comparing the first N characters')
            [CompletionResult]::new('--skip-chars', 'skip-chars', [CompletionResultType]::ParameterName, 'avoid comparing the first N characters')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'avoid comparing the first N fields')
            [CompletionResult]::new('--skip-fields', 'skip-fields', [CompletionResultType]::ParameterName, 'avoid comparing the first N fields')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'prefix lines by the number of occurrences')
            [CompletionResult]::new('--count', 'count', [CompletionResultType]::ParameterName, 'prefix lines by the number of occurrences')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'ignore differences in case when comparing')
            [CompletionResult]::new('--ignore-case', 'ignore-case', [CompletionResultType]::ParameterName, 'ignore differences in case when comparing')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'only print duplicate lines')
            [CompletionResult]::new('--repeated', 'repeated', [CompletionResultType]::ParameterName, 'only print duplicate lines')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'only print unique lines')
            [CompletionResult]::new('--unique', 'unique', [CompletionResultType]::ParameterName, 'only print unique lines')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'end lines with 0 byte, not newline')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'end lines with 0 byte, not newline')
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
