
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_truncate' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_truncate'
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
        'uu_truncate' {
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'base the size of each file on the size of RFILE')
            [CompletionResult]::new('--reference', 'reference', [CompletionResultType]::ParameterName, 'base the size of each file on the size of RFILE')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified')
            [CompletionResult]::new('--size', 'size', [CompletionResultType]::ParameterName, 'set or adjust the size of each file according to SIZE, which is in bytes unless --io-blocks is specified')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)')
            [CompletionResult]::new('--io-blocks', 'io-blocks', [CompletionResultType]::ParameterName, 'treat SIZE as the number of I/O blocks of the file rather than bytes (NOT IMPLEMENTED)')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'do not create files that do not exist')
            [CompletionResult]::new('--no-create', 'no-create', [CompletionResultType]::ParameterName, 'do not create files that do not exist')
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
