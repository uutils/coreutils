
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_uname' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_uname'
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
        'uu_uname' {
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Behave as though all of the options -mnrsvo were specified.')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'Behave as though all of the options -mnrsvo were specified.')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'print the kernel name.')
            [CompletionResult]::new('--kernel-name', 'kernel-name', [CompletionResultType]::ParameterName, 'print the kernel name.')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'print the nodename (the nodename may be a name that the system is known by to a communications network).')
            [CompletionResult]::new('--nodename', 'nodename', [CompletionResultType]::ParameterName, 'print the nodename (the nodename may be a name that the system is known by to a communications network).')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'print the operating system release.')
            [CompletionResult]::new('--kernel-release', 'kernel-release', [CompletionResultType]::ParameterName, 'print the operating system release.')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'print the operating system version.')
            [CompletionResult]::new('--kernel-version', 'kernel-version', [CompletionResultType]::ParameterName, 'print the operating system version.')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'print the machine hardware name.')
            [CompletionResult]::new('--machine', 'machine', [CompletionResultType]::ParameterName, 'print the machine hardware name.')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'print the operating system name.')
            [CompletionResult]::new('--operating-system', 'operating-system', [CompletionResultType]::ParameterName, 'print the operating system name.')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'print the processor type (non-portable)')
            [CompletionResult]::new('--processor', 'processor', [CompletionResultType]::ParameterName, 'print the processor type (non-portable)')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'print the hardware platform (non-portable)')
            [CompletionResult]::new('--hardware-platform', 'hardware-platform', [CompletionResultType]::ParameterName, 'print the hardware platform (non-portable)')
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
