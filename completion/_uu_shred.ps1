
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_shred' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_shred'
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
        'uu_shred' {
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'overwrite N times instead of the default (3)')
            [CompletionResult]::new('--iterations', 'iterations', [CompletionResultType]::ParameterName, 'overwrite N times instead of the default (3)')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'shred this many bytes (suffixes like K, M, G accepted)')
            [CompletionResult]::new('--size', 'size', [CompletionResultType]::ParameterName, 'shred this many bytes (suffixes like K, M, G accepted)')
            [CompletionResult]::new('--remove', 'remove', [CompletionResultType]::ParameterName, 'like -u but give control on HOW to delete;  See below')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'change permissions to allow writing if necessary')
            [CompletionResult]::new('--force', 'force', [CompletionResultType]::ParameterName, 'change permissions to allow writing if necessary')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'deallocate and remove file after overwriting')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'show progress')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'show progress')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'do not round file sizes up to the next full block;
this is the default for non-regular files')
            [CompletionResult]::new('--exact', 'exact', [CompletionResultType]::ParameterName, 'do not round file sizes up to the next full block;
this is the default for non-regular files')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'add a final overwrite with zeros to hide shredding')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'add a final overwrite with zeros to hide shredding')
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
