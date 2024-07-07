
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_numfmt' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_numfmt'
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
        'uu_numfmt' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'use X instead of whitespace for field delimiter')
            [CompletionResult]::new('--delimiter', 'delimiter', [CompletionResultType]::ParameterName, 'use X instead of whitespace for field delimiter')
            [CompletionResult]::new('--field', 'field', [CompletionResultType]::ParameterName, 'replace the numbers in these input fields; see FIELDS below')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'use printf style floating-point FORMAT; see FORMAT below for details')
            [CompletionResult]::new('--from', 'from', [CompletionResultType]::ParameterName, 'auto-scale input numbers to UNITs; see UNIT below')
            [CompletionResult]::new('--from-unit', 'from-unit', [CompletionResultType]::ParameterName, 'specify the input unit size')
            [CompletionResult]::new('--to', 'to', [CompletionResultType]::ParameterName, 'auto-scale output numbers to UNITs; see UNIT below')
            [CompletionResult]::new('--to-unit', 'to-unit', [CompletionResultType]::ParameterName, 'the output unit size')
            [CompletionResult]::new('--padding', 'padding', [CompletionResultType]::ParameterName, 'pad the output to N characters; positive N will right-align; negative N will left-align; padding is ignored if the output is wider than N; the default is to automatically pad if a whitespace is found')
            [CompletionResult]::new('--header', 'header', [CompletionResultType]::ParameterName, 'print (without converting) the first N header lines; N defaults to 1 if not specified')
            [CompletionResult]::new('--round', 'round', [CompletionResultType]::ParameterName, 'use METHOD for rounding when scaling')
            [CompletionResult]::new('--suffix', 'suffix', [CompletionResultType]::ParameterName, 'print SUFFIX after each formatted number, and accept inputs optionally ending with SUFFIX')
            [CompletionResult]::new('--invalid', 'invalid', [CompletionResultType]::ParameterName, 'set the failure mode for invalid input')
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
