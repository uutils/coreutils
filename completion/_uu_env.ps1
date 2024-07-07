
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_env' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_env'
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
        'uu_env' {
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'change working directory to DIR')
            [CompletionResult]::new('--chdir', 'chdir', [CompletionResultType]::ParameterName, 'change working directory to DIR')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'read and set variables from a ".env"-style configuration file (prior to any unset and/or set)')
            [CompletionResult]::new('--file', 'file', [CompletionResultType]::ParameterName, 'read and set variables from a ".env"-style configuration file (prior to any unset and/or set)')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'remove variable from the environment')
            [CompletionResult]::new('--unset', 'unset', [CompletionResultType]::ParameterName, 'remove variable from the environment')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'process and split S into separate arguments; used to pass multiple arguments on shebang lines')
            [CompletionResult]::new('--split-string', 'split-string', [CompletionResultType]::ParameterName, 'process and split S into separate arguments; used to pass multiple arguments on shebang lines')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.')
            [CompletionResult]::new('--argv0', 'argv0', [CompletionResultType]::ParameterName, 'Override the zeroth argument passed to the command being executed. Without this option a default value of `command` is used.')
            [CompletionResult]::new('--ignore-signal', 'ignore-signal', [CompletionResultType]::ParameterName, 'set handling of SIG signal(s) to do nothing')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'start with an empty environment')
            [CompletionResult]::new('--ignore-environment', 'ignore-environment', [CompletionResultType]::ParameterName, 'start with an empty environment')
            [CompletionResult]::new('-0', '0', [CompletionResultType]::ParameterName, 'end each output line with a 0 byte rather than a newline (only valid when printing the environment)')
            [CompletionResult]::new('--null', 'null', [CompletionResultType]::ParameterName, 'end each output line with a 0 byte rather than a newline (only valid when printing the environment)')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'print verbose information for each processing step')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'print verbose information for each processing step')
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
