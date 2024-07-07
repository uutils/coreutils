
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_cp' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_cp'
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
        'uu_cp' {
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'copy all SOURCE arguments into target-directory')
            [CompletionResult]::new('--target-directory', 'target-directory', [CompletionResultType]::ParameterName, 'copy all SOURCE arguments into target-directory')
            [CompletionResult]::new('--backup', 'backup', [CompletionResultType]::ParameterName, 'make a backup of each existing destination file')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'override the usual backup suffix')
            [CompletionResult]::new('--update', 'update', [CompletionResultType]::ParameterName, 'move only when the SOURCE file is newer than the destination file or when the destination file is missing')
            [CompletionResult]::new('--reflink', 'reflink', [CompletionResultType]::ParameterName, 'control clone/CoW copies. See below')
            [CompletionResult]::new('--preserve', 'preserve', [CompletionResultType]::ParameterName, 'Preserve the specified attributes (default: mode, ownership (unix only), timestamps), if possible additional attributes: context, links, xattr, all')
            [CompletionResult]::new('--no-preserve', 'no-preserve', [CompletionResultType]::ParameterName, 'don''t preserve the specified attributes')
            [CompletionResult]::new('--sparse', 'sparse', [CompletionResultType]::ParameterName, 'control creation of sparse files. See below')
            [CompletionResult]::new('--context', 'context', [CompletionResultType]::ParameterName, 'NotImplemented: set SELinux security context of destination file to default type')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'Treat DEST as a regular file and not a directory')
            [CompletionResult]::new('--no-target-directory', 'no-target-directory', [CompletionResultType]::ParameterName, 'Treat DEST as a regular file and not a directory')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'ask before overwriting files')
            [CompletionResult]::new('--interactive', 'interactive', [CompletionResultType]::ParameterName, 'ask before overwriting files')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'hard-link files instead of copying')
            [CompletionResult]::new('--link', 'link', [CompletionResultType]::ParameterName, 'hard-link files instead of copying')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'don''t overwrite a file that already exists')
            [CompletionResult]::new('--no-clobber', 'no-clobber', [CompletionResultType]::ParameterName, 'don''t overwrite a file that already exists')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'copy directories recursively')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'copy directories recursively')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'copy directories recursively')
            [CompletionResult]::new('--strip-trailing-slashes', 'strip-trailing-slashes', [CompletionResultType]::ParameterName, 'remove any trailing slashes from each SOURCE argument')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'explain how a file is copied. Implies -v')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'explicitly state what is being done')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'explicitly state what is being done')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'make symbolic links instead of copying')
            [CompletionResult]::new('--symbolic-link', 'symbolic-link', [CompletionResultType]::ParameterName, 'make symbolic links instead of copying')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.')
            [CompletionResult]::new('--force', 'force', [CompletionResultType]::ParameterName, 'if an existing destination file cannot be opened, remove it and try again (this option is ignored when the -n option is also used). Currently not implemented for Windows.')
            [CompletionResult]::new('--remove-destination', 'remove-destination', [CompletionResultType]::ParameterName, 'remove each existing destination file before attempting to open it (contrast with --force). On Windows, currently only works for writeable files.')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'like --backup but does not accept an argument')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'like --update but does not accept an argument')
            [CompletionResult]::new('--attributes-only', 'attributes-only', [CompletionResultType]::ParameterName, 'Don''t copy the file data, just the attributes')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'same as --preserve=mode,ownership(unix only),timestamps')
            [CompletionResult]::new('--preserve-default-attributes', 'preserve-default-attributes', [CompletionResultType]::ParameterName, 'same as --preserve=mode,ownership(unix only),timestamps')
            [CompletionResult]::new('--parents', 'parents', [CompletionResultType]::ParameterName, 'use full source file name under DIRECTORY')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'never follow symbolic links in SOURCE')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'never follow symbolic links in SOURCE')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'always follow symbolic links in SOURCE')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'always follow symbolic links in SOURCE')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'follow command-line symbolic links in SOURCE')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Same as -dR --preserve=all')
            [CompletionResult]::new('--archive', 'archive', [CompletionResultType]::ParameterName, 'Same as -dR --preserve=all')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'same as --no-dereference --preserve=links')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'stay on this file system')
            [CompletionResult]::new('--one-file-system', 'one-file-system', [CompletionResultType]::ParameterName, 'stay on this file system')
            [CompletionResult]::new('--copy-contents', 'copy-contents', [CompletionResultType]::ParameterName, 'NotImplemented: copy contents of special files when recursive')
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
