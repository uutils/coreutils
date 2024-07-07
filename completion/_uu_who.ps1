
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_who' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_who'
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
        'uu_who' {
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'same as -b -d --login -p -r -t -T -u')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'same as -b -d --login -p -r -t -T -u')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'time of last system boot')
            [CompletionResult]::new('--boot', 'boot', [CompletionResultType]::ParameterName, 'time of last system boot')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'print dead processes')
            [CompletionResult]::new('--dead', 'dead', [CompletionResultType]::ParameterName, 'print dead processes')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'print line of column headings')
            [CompletionResult]::new('--heading', 'heading', [CompletionResultType]::ParameterName, 'print line of column headings')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'print system login processes')
            [CompletionResult]::new('--login', 'login', [CompletionResultType]::ParameterName, 'print system login processes')
            [CompletionResult]::new('--lookup', 'lookup', [CompletionResultType]::ParameterName, 'attempt to canonicalize hostnames via DNS')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'only hostname and user associated with stdin')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'print active processes spawned by init')
            [CompletionResult]::new('--process', 'process', [CompletionResultType]::ParameterName, 'print active processes spawned by init')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'all login names and number of users logged on')
            [CompletionResult]::new('--count', 'count', [CompletionResultType]::ParameterName, 'all login names and number of users logged on')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'print current runlevel')
            [CompletionResult]::new('--runlevel', 'runlevel', [CompletionResultType]::ParameterName, 'print current runlevel')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'print only name, line, and time (default)')
            [CompletionResult]::new('--short', 'short', [CompletionResultType]::ParameterName, 'print only name, line, and time (default)')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'print last system clock change')
            [CompletionResult]::new('--time', 'time', [CompletionResultType]::ParameterName, 'print last system clock change')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'list users logged in')
            [CompletionResult]::new('--users', 'users', [CompletionResultType]::ParameterName, 'list users logged in')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'add user''s message status as +, - or ?')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'add user''s message status as +, - or ?')
            [CompletionResult]::new('--mesg', 'mesg', [CompletionResultType]::ParameterName, 'add user''s message status as +, - or ?')
            [CompletionResult]::new('--message', 'message', [CompletionResultType]::ParameterName, 'add user''s message status as +, - or ?')
            [CompletionResult]::new('--writable', 'writable', [CompletionResultType]::ParameterName, 'add user''s message status as +, - or ?')
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
