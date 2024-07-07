
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_chown' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_chown'
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
        'uu_chown' {
            [CompletionResult]::new('--from', 'from', [CompletionResultType]::ParameterName, 'change the owner and/or group of each file only if its current owner and/or group match those specified here. Either may be omitted, in which case a match is not required for the omitted attribute')
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'use RFILE''s owner and group rather than specifying OWNER:GROUP values')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'like verbose but report only when a change is made')
            [CompletionResult]::new('--changes', 'changes', [CompletionResultType]::ParameterName, 'like verbose but report only when a change is made')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'affect the referent of each symbolic link (this is the default), rather than the symbolic link itself')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'affect symbolic links instead of any referenced file (useful only on systems that can change the ownership of a symlink)')
            [CompletionResult]::new('--preserve-root', 'preserve-root', [CompletionResultType]::ParameterName, 'fail to operate recursively on ''/''')
            [CompletionResult]::new('--no-preserve-root', 'no-preserve-root', [CompletionResultType]::ParameterName, 'do not treat ''/'' specially (the default)')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'operate on files and directories recursively')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'operate on files and directories recursively')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'f')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'silent')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'if a command line argument is a symbolic link to a directory, traverse it')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'traverse every symbolic link to a directory encountered')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'do not traverse any symbolic links (default)')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'output a diagnostic for every file processed')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'output a diagnostic for every file processed')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
