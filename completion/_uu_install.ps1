
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_install' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_install'
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
        'uu_install' {
            [CompletionResult]::new('--backup', 'backup', [CompletionResultType]::ParameterName, 'make a backup of each existing destination file')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'set group ownership, instead of process''s current group')
            [CompletionResult]::new('--group', 'group', [CompletionResultType]::ParameterName, 'set group ownership, instead of process''s current group')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'set permission mode (as in chmod), instead of rwxr-xr-x')
            [CompletionResult]::new('--mode', 'mode', [CompletionResultType]::ParameterName, 'set permission mode (as in chmod), instead of rwxr-xr-x')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'set ownership (super-user only)')
            [CompletionResult]::new('--owner', 'owner', [CompletionResultType]::ParameterName, 'set ownership (super-user only)')
            [CompletionResult]::new('--strip-program', 'strip-program', [CompletionResultType]::ParameterName, 'program used to strip binaries (no action Windows)')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'move all SOURCE arguments into DIRECTORY')
            [CompletionResult]::new('--target-directory', 'target-directory', [CompletionResultType]::ParameterName, 'move all SOURCE arguments into DIRECTORY')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'like --backup but does not accept an argument')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ignored')
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'compare each pair of source and destination files, and in some cases, do not modify the destination at all')
            [CompletionResult]::new('--compare', 'compare', [CompletionResultType]::ParameterName, 'compare each pair of source and destination files, and in some cases, do not modify the destination at all')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'treat all arguments as directory names. create all components of the specified directories')
            [CompletionResult]::new('--directory', 'directory', [CompletionResultType]::ParameterName, 'treat all arguments as directory names. create all components of the specified directories')
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'create all leading components of DEST except the last, then copy SOURCE to DEST')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'apply access/modification times of SOURCE files to corresponding destination files')
            [CompletionResult]::new('--preserve-timestamps', 'preserve-timestamps', [CompletionResultType]::ParameterName, 'apply access/modification times of SOURCE files to corresponding destination files')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'strip symbol tables (no action Windows)')
            [CompletionResult]::new('--strip', 'strip', [CompletionResultType]::ParameterName, 'strip symbol tables (no action Windows)')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, '(unimplemented) treat DEST as a normal file')
            [CompletionResult]::new('--no-target-directory', 'no-target-directory', [CompletionResultType]::ParameterName, '(unimplemented) treat DEST as a normal file')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, '(unimplemented) preserve security context')
            [CompletionResult]::new('--preserve-context', 'preserve-context', [CompletionResultType]::ParameterName, '(unimplemented) preserve security context')
            [CompletionResult]::new('-Z', 'Z ', [CompletionResultType]::ParameterName, '(unimplemented) set security context of files and directories')
            [CompletionResult]::new('--context', 'context', [CompletionResultType]::ParameterName, '(unimplemented) set security context of files and directories')
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
