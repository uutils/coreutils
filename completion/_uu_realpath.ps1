
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_realpath' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_realpath'
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
        'uu_realpath' {
            [CompletionResult]::new('--relative-to', 'relative-to', [CompletionResultType]::ParameterName, 'print the resolved path relative to DIR')
            [CompletionResult]::new('--relative-base', 'relative-base', [CompletionResultType]::ParameterName, 'print absolute paths unless paths below DIR')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Do not print warnings for invalid paths')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'Do not print warnings for invalid paths')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links')
            [CompletionResult]::new('--strip', 'strip', [CompletionResultType]::ParameterName, 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links')
            [CompletionResult]::new('--no-symlinks', 'no-symlinks', [CompletionResultType]::ParameterName, 'Only strip ''.'' and ''..'' components, but don''t resolve symbolic links')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'Separate output filenames with \0 rather than newline')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'Separate output filenames with \0 rather than newline')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'resolve ''..'' components before symlinks')
            [CompletionResult]::new('--logical', 'logical', [CompletionResultType]::ParameterName, 'resolve ''..'' components before symlinks')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'resolve symlinks as encountered (default)')
            [CompletionResult]::new('--physical', 'physical', [CompletionResultType]::ParameterName, 'resolve symlinks as encountered (default)')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, all components must exist')
            [CompletionResult]::new('--canonicalize-existing', 'canonicalize-existing', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, all components must exist')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence')
            [CompletionResult]::new('--canonicalize-missing', 'canonicalize-missing', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence')
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
