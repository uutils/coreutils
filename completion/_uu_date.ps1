
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_date' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_date'
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
        'uu_date' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'display time described by STRING, not ''now''')
            [CompletionResult]::new('--date', 'date', [CompletionResultType]::ParameterName, 'display time described by STRING, not ''now''')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'like --date; once for each line of DATEFILE')
            [CompletionResult]::new('--file', 'file', [CompletionResultType]::ParameterName, 'like --date; once for each line of DATEFILE')
            [CompletionResult]::new('-I', 'I ', [CompletionResultType]::ParameterName, 'output date/time in ISO 8601 format.
FMT=''date'' for date only (the default),
''hours'', ''minutes'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00')
            [CompletionResult]::new('--iso-8601', 'iso-8601', [CompletionResultType]::ParameterName, 'output date/time in ISO 8601 format.
FMT=''date'' for date only (the default),
''hours'', ''minutes'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14T02:34:56-06:00')
            [CompletionResult]::new('--rfc-3339', 'rfc-3339', [CompletionResultType]::ParameterName, 'output date/time in RFC 3339 format.
FMT=''date'', ''seconds'', or ''ns''
for date and time to the indicated precision.
Example: 2006-08-14 02:34:56-06:00')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'display the last modification time of FILE')
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'display the last modification time of FILE')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'set time described by STRING')
            [CompletionResult]::new('--set', 'set', [CompletionResultType]::ParameterName, 'set time described by STRING')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600')
            [CompletionResult]::new('--rfc-email', 'rfc-email', [CompletionResultType]::ParameterName, 'output date and time in RFC 5322 format.
Example: Mon, 14 Aug 2006 02:34:56 -0600')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'annotate the parsed date, and warn about questionable usage to stderr')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'print or set Coordinated Universal Time (UTC)')
            [CompletionResult]::new('--universal', 'universal', [CompletionResultType]::ParameterName, 'print or set Coordinated Universal Time (UTC)')
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
