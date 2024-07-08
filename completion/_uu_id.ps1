
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_id' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_id'
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
        'uu_id' {
            [CompletionResult]::new('-A', 'A ', [CompletionResultType]::ParameterName, 'Display the process audit user ID and other process audit properties,
which requires privilege (not available on Linux).')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'Display only the effective user ID as a number.')
            [CompletionResult]::new('--user', 'user', [CompletionResultType]::ParameterName, 'Display only the effective user ID as a number.')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Display only the effective group ID as a number')
            [CompletionResult]::new('--group', 'group', [CompletionResultType]::ParameterName, 'Display only the effective group ID as a number')
            [CompletionResult]::new('-G', 'G ', [CompletionResultType]::ParameterName, 'Display only the different group IDs as white-space separated numbers, in no particular order.')
            [CompletionResult]::new('--groups', 'groups', [CompletionResultType]::ParameterName, 'Display only the different group IDs as white-space separated numbers, in no particular order.')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'Make the output human-readable. Each display is on a separate line.')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Display the name of the user or group ID for the -G, -g and -u options instead of the number.
If any of the ID numbers cannot be mapped into names, the number will be displayed as usual.')
            [CompletionResult]::new('--name', 'name', [CompletionResultType]::ParameterName, 'Display the name of the user or group ID for the -G, -g and -u options instead of the number.
If any of the ID numbers cannot be mapped into names, the number will be displayed as usual.')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'Display the id as a password file entry.')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'Display the real ID for the -G, -g and -u options instead of the effective ID.')
            [CompletionResult]::new('--real', 'real', [CompletionResultType]::ParameterName, 'Display the real ID for the -G, -g and -u options instead of the effective ID.')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'delimit entries with NUL characters, not whitespace;
not permitted in default format')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'delimit entries with NUL characters, not whitespace;
not permitted in default format')
            [CompletionResult]::new('-Z', 'Z ', [CompletionResultType]::ParameterName, 'print only the security context of the process')
            [CompletionResult]::new('--context', 'context', [CompletionResultType]::ParameterName, 'print only the security context of the process')
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
