
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_touch' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_touch'
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
        'uu_touch' {
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'use [[CC]YY]MMDDhhmm[.ss] instead of the current time')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'parse argument and use it instead of current time')
            [CompletionResult]::new('--date', 'date', [CompletionResultType]::ParameterName, 'parse argument and use it instead of current time')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'use this file''s times instead of the current time')
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'use this file''s times instead of the current time')
            [CompletionResult]::new('--time', 'time', [CompletionResultType]::ParameterName, 'change only the specified time: "access", "atime", or "use" are equivalent to -a; "modify" or "mtime" are equivalent to -m')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'change only the access time')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'change only the modification time')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'do not create any files')
            [CompletionResult]::new('--no-create', 'no-create', [CompletionResultType]::ParameterName, 'do not create any files')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
