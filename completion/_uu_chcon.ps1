
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_chcon' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_chcon'
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
        'uu_chcon' {
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'Use security context of RFILE, rather than specifying a CONTEXT value.')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Set user USER in the target security context.')
            [CompletionResult]::new('--user', 'user', [CompletionResultType]::ParameterName, 'Set user USER in the target security context.')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Set role ROLE in the target security context.')
            [CompletionResult]::new('--role', 'role', [CompletionResultType]::ParameterName, 'Set role ROLE in the target security context.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'Set type TYPE in the target security context.')
            [CompletionResult]::new('--type', 'type', [CompletionResultType]::ParameterName, 'Set type TYPE in the target security context.')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'Set range RANGE in the target security context.')
            [CompletionResult]::new('--range', 'range', [CompletionResultType]::ParameterName, 'Set range RANGE in the target security context.')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'Affect the referent of each symbolic link (this is the default), rather than the symbolic link itself.')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'Affect symbolic links instead of any referenced file.')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'Affect symbolic links instead of any referenced file.')
            [CompletionResult]::new('--preserve-root', 'preserve-root', [CompletionResultType]::ParameterName, 'Fail to operate recursively on ''/''.')
            [CompletionResult]::new('--no-preserve-root', 'no-preserve-root', [CompletionResultType]::ParameterName, 'Do not treat ''/'' specially (the default).')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'Operate on files and directories recursively.')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'Operate on files and directories recursively.')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'If a command line argument is a symbolic link to a directory, traverse it. Only valid when -R is specified.')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'Traverse every symbolic link to a directory encountered. Only valid when -R is specified.')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'Do not traverse any symbolic links (default). Only valid when -R is specified.')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Output a diagnostic for every file processed.')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Output a diagnostic for every file processed.')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
