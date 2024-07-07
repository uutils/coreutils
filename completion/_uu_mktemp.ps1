
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_mktemp' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_mktemp'
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
        'uu_mktemp' {
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'append SUFFIX to TEMPLATE; SUFFIX must not contain a path separator. This option is implied if TEMPLATE does not end with X.')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'short form of --tmpdir')
            [CompletionResult]::new('--tmpdir', 'tmpdir', [CompletionResultType]::ParameterName, 'interpret TEMPLATE relative to DIR; if DIR is not specified, use $TMPDIR ($TMP on windows) if set, else /tmp. With this option, TEMPLATE must not be an absolute name; unlike with -t, TEMPLATE may contain slashes, but mktemp creates only the final component')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Make a directory instead of a file')
            [CompletionResult]::new('--directory', 'directory', [CompletionResultType]::ParameterName, 'Make a directory instead of a file')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'do not create anything; merely print a name (unsafe)')
            [CompletionResult]::new('--dry-run', 'dry-run', [CompletionResultType]::ParameterName, 'do not create anything; merely print a name (unsafe)')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Fail silently if an error occurs.')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'Fail silently if an error occurs.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Generate a template (using the supplied prefix and TMPDIR (TMP on windows) if set) to create a filename template [deprecated]')
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
