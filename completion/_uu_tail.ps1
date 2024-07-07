
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_tail' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_tail'
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
        'uu_tail' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'Number of bytes to print')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'Number of bytes to print')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'Print the file as it grows')
            [CompletionResult]::new('--follow', 'follow', [CompletionResultType]::ParameterName, 'Print the file as it grows')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'Number of lines to print')
            [CompletionResult]::new('--lines', 'lines', [CompletionResultType]::ParameterName, 'Number of lines to print')
            [CompletionResult]::new('--pid', 'pid', [CompletionResultType]::ParameterName, 'With -f, terminate after process ID, PID dies')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Number of seconds to sleep between polling the file when running with -f')
            [CompletionResult]::new('--sleep-interval', 'sleep-interval', [CompletionResultType]::ParameterName, 'Number of seconds to sleep between polling the file when running with -f')
            [CompletionResult]::new('--max-unchanged-stats', 'max-unchanged-stats', [CompletionResultType]::ParameterName, 'Reopen a FILE which has not changed size after N (default 5) iterations to see if it has been unlinked or renamed (this is the usual case of rotated log files); This option is meaningful only when polling (i.e., with --use-polling) and when --follow=name')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'Never output headers giving file names')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'Never output headers giving file names')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'Never output headers giving file names')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'Always output headers giving file names')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'Always output headers giving file names')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'Line delimiter is NUL, not newline')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'Line delimiter is NUL, not newline')
            [CompletionResult]::new('--use-polling', 'use-polling', [CompletionResultType]::ParameterName, 'Disable ''inotify'' support and use polling instead')
            [CompletionResult]::new('--retry', 'retry', [CompletionResultType]::ParameterName, 'Keep trying to open a file if it is inaccessible')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'Same as --follow=name --retry')
            [CompletionResult]::new('--presume-input-pipe', 'presume-input-pipe', [CompletionResultType]::ParameterName, 'presume-input-pipe')
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
