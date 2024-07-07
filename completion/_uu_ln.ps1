
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_ln' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_ln'
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
        'uu_ln' {
            [CompletionResult]::new('--backup', 'backup', [CompletionResultType]::ParameterName, 'make a backup of each existing destination file')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'specify the DIRECTORY in which to create the links')
            [CompletionResult]::new('--target-directory', 'target-directory', [CompletionResultType]::ParameterName, 'specify the DIRECTORY in which to create the links')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'like --backup but does not accept an argument')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'remove existing destination files')
            [CompletionResult]::new('--force', 'force', [CompletionResultType]::ParameterName, 'remove existing destination files')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'prompt whether to remove existing destination files')
            [CompletionResult]::new('--interactive', 'interactive', [CompletionResultType]::ParameterName, 'prompt whether to remove existing destination files')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'treat LINK_NAME as a normal file if it is a symbolic link to a directory')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'treat LINK_NAME as a normal file if it is a symbolic link to a directory')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'follow TARGETs that are symbolic links')
            [CompletionResult]::new('--logical', 'logical', [CompletionResultType]::ParameterName, 'follow TARGETs that are symbolic links')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'make hard links directly to symbolic links')
            [CompletionResult]::new('--physical', 'physical', [CompletionResultType]::ParameterName, 'make hard links directly to symbolic links')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'make symbolic links instead of hard links')
            [CompletionResult]::new('--symbolic', 'symbolic', [CompletionResultType]::ParameterName, 'make symbolic links instead of hard links')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'treat LINK_NAME as a normal file always')
            [CompletionResult]::new('--no-target-directory', 'no-target-directory', [CompletionResultType]::ParameterName, 'treat LINK_NAME as a normal file always')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'create symbolic links relative to link location')
            [CompletionResult]::new('--relative', 'relative', [CompletionResultType]::ParameterName, 'create symbolic links relative to link location')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'print name of each linked file')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'print name of each linked file')
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
