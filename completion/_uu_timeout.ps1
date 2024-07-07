
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_timeout' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_timeout'
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
        'uu_timeout' {
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'also send a KILL signal if COMMAND is still running this long after the initial signal was sent')
            [CompletionResult]::new('--kill-after', 'kill-after', [CompletionResultType]::ParameterName, 'also send a KILL signal if COMMAND is still running this long after the initial signal was sent')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'specify the signal to be sent on timeout; SIGNAL may be a name like ''HUP'' or a number; see ''kill -l'' for a list of signals')
            [CompletionResult]::new('--signal', 'signal', [CompletionResultType]::ParameterName, 'specify the signal to be sent on timeout; SIGNAL may be a name like ''HUP'' or a number; see ''kill -l'' for a list of signals')
            [CompletionResult]::new('--foreground', 'foreground', [CompletionResultType]::ParameterName, 'when not running timeout directly from a shell prompt, allow COMMAND to read from the TTY and get TTY signals; in this mode, children of COMMAND will not be timed out')
            [CompletionResult]::new('--preserve-status', 'preserve-status', [CompletionResultType]::ParameterName, 'exit with the same status as COMMAND, even when the command times out')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'diagnose to stderr any signal sent upon timeout')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'diagnose to stderr any signal sent upon timeout')
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
