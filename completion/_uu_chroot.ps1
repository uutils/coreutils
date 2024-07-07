
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_chroot' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_chroot'
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
        'uu_chroot' {
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'User (ID or name) to switch before running the program')
            [CompletionResult]::new('--user', 'user', [CompletionResultType]::ParameterName, 'User (ID or name) to switch before running the program')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'Group (ID or name) to switch to')
            [CompletionResult]::new('--group', 'group', [CompletionResultType]::ParameterName, 'Group (ID or name) to switch to')
            [CompletionResult]::new('-G', 'G ', [CompletionResultType]::ParameterName, 'Comma-separated list of groups to switch to')
            [CompletionResult]::new('--groups', 'groups', [CompletionResultType]::ParameterName, 'Comma-separated list of groups to switch to')
            [CompletionResult]::new('--userspec', 'userspec', [CompletionResultType]::ParameterName, 'Colon-separated user and group to switch to. Same as -u USER -g GROUP. Userspec has higher preference than -u and/or -g')
            [CompletionResult]::new('--skip-chdir', 'skip-chdir', [CompletionResultType]::ParameterName, 'Use this option to not change the working directory to / after changing the root directory to newroot, i.e., inside the chroot.')
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
