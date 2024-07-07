
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_chmod' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_chmod'
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
        'uu_chmod' {
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'use RFILE''s mode instead of MODE values')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'like verbose but report only when a change is made')
            [CompletionResult]::new('--changes', 'changes', [CompletionResultType]::ParameterName, 'like verbose but report only when a change is made')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'output a diagnostic for every file processed')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'output a diagnostic for every file processed')
            [CompletionResult]::new('--no-preserve-root', 'no-preserve-root', [CompletionResultType]::ParameterName, 'do not treat ''/'' specially (the default)')
            [CompletionResult]::new('--preserve-root', 'preserve-root', [CompletionResultType]::ParameterName, 'fail to operate recursively on ''/''')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'change files and directories recursively')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'change files and directories recursively')
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
