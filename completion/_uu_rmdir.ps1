
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_rmdir' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_rmdir'
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
        'uu_rmdir' {
            [CompletionResult]::new('--ignore-fail-on-non-empty', 'ignore-fail-on-non-empty', [CompletionResultType]::ParameterName, 'ignore each failure that is solely because a directory is non-empty')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'remove DIRECTORY and its ancestors; e.g.,
                  ''rmdir -p a/b/c'' is similar to rmdir a/b/c a/b a')
            [CompletionResult]::new('--parents', 'parents', [CompletionResultType]::ParameterName, 'remove DIRECTORY and its ancestors; e.g.,
                  ''rmdir -p a/b/c'' is similar to rmdir a/b/c a/b a')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'output a diagnostic for every directory processed')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'output a diagnostic for every directory processed')
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
