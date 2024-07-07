
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_tr' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_tr'
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
        'uu_tr' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'use the complement of SET1')
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'use the complement of SET1')
            [CompletionResult]::new('--complement', 'complement', [CompletionResultType]::ParameterName, 'use the complement of SET1')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'delete characters in SET1, do not translate')
            [CompletionResult]::new('--delete', 'delete', [CompletionResultType]::ParameterName, 'delete characters in SET1, do not translate')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character')
            [CompletionResult]::new('--squeeze-repeats', 'squeeze-repeats', [CompletionResultType]::ParameterName, 'replace each sequence of a repeated character that is listed in the last specified SET, with a single occurrence of that character')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'first truncate SET1 to length of SET2')
            [CompletionResult]::new('--truncate-set1', 'truncate-set1', [CompletionResultType]::ParameterName, 'first truncate SET1 to length of SET2')
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
