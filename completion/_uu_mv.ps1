
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_mv' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_mv'
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
        'uu_mv' {
            [CompletionResult]::new('--backup', 'backup', [CompletionResultType]::ParameterName, 'make a backup of each existing destination file')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--update', 'update', [CompletionResultType]::ParameterName, 'move only when the SOURCE file is newer than the destination file or when the destination file is missing')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'move all SOURCE arguments into DIRECTORY')
            [CompletionResult]::new('--target-directory', 'target-directory', [CompletionResultType]::ParameterName, 'move all SOURCE arguments into DIRECTORY')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'do not prompt before overwriting')
            [CompletionResult]::new('--force', 'force', [CompletionResultType]::ParameterName, 'do not prompt before overwriting')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'prompt before override')
            [CompletionResult]::new('--interactive', 'interactive', [CompletionResultType]::ParameterName, 'prompt before override')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'do not overwrite an existing file')
            [CompletionResult]::new('--no-clobber', 'no-clobber', [CompletionResultType]::ParameterName, 'do not overwrite an existing file')
            [CompletionResult]::new('--strip-trailing-slashes', 'strip-trailing-slashes', [CompletionResultType]::ParameterName, 'remove any trailing slashes from each SOURCE argument')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'like --backup but does not accept an argument')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'like --update but does not accept an argument')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'treat DEST as a normal file')
            [CompletionResult]::new('--no-target-directory', 'no-target-directory', [CompletionResultType]::ParameterName, 'treat DEST as a normal file')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.')
            [CompletionResult]::new('--progress', 'progress', [CompletionResultType]::ParameterName, 'Display a progress bar. 
Note: this feature is not supported by GNU coreutils.')
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
