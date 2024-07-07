
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'sha3-384sum' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'sha3-384sum'
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
        'sha3-384sum' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'read in binary mode')
            [CompletionResult]::new('--binary', 'binary', [CompletionResultType]::ParameterName, 'read in binary mode')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'read hashsums from the FILEs and check them')
            [CompletionResult]::new('--check', 'check', [CompletionResultType]::ParameterName, 'read hashsums from the FILEs and check them')
            [CompletionResult]::new('--tag', 'tag', [CompletionResultType]::ParameterName, 'create a BSD-style checksum')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'read in text mode (default)')
            [CompletionResult]::new('--text', 'text', [CompletionResultType]::ParameterName, 'read in text mode (default)')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'don''t print OK for each successfully verified file')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'don''t print OK for each successfully verified file')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'don''t output anything, status code shows success')
            [CompletionResult]::new('--status', 'status', [CompletionResultType]::ParameterName, 'don''t output anything, status code shows success')
            [CompletionResult]::new('--strict', 'strict', [CompletionResultType]::ParameterName, 'exit non-zero for improperly formatted checksum lines')
            [CompletionResult]::new('--ignore-missing', 'ignore-missing', [CompletionResultType]::ParameterName, 'don''t fail or report status for missing files')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'warn about improperly formatted checksum lines')
            [CompletionResult]::new('--warn', 'warn', [CompletionResultType]::ParameterName, 'warn about improperly formatted checksum lines')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'end each output line with NUL, not newline')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'end each output line with NUL, not newline')
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
