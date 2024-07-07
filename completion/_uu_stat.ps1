
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_stat' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_stat'
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
        'uu_stat' {
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'use the specified FORMAT instead of the default;
 output a newline after each use of FORMAT')
            [CompletionResult]::new('--printf', 'printf', [CompletionResultType]::ParameterName, 'like --format, but interpret backslash escapes,
            and do not output a mandatory trailing newline;
            if you want a newline, include 
 in FORMAT')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'follow links')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'follow links')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'display file system status instead of file status')
            [CompletionResult]::new('--file-system', 'file-system', [CompletionResultType]::ParameterName, 'display file system status instead of file status')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'print the information in terse form')
            [CompletionResult]::new('--terse', 'terse', [CompletionResultType]::ParameterName, 'print the information in terse form')
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
