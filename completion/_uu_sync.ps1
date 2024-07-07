
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_sync' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_sync'
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
        'uu_sync' {
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'sync the file systems that contain the files (Linux and Windows only)')
            [CompletionResult]::new('--file-system', 'file-system', [CompletionResultType]::ParameterName, 'sync the file systems that contain the files (Linux and Windows only)')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'sync only file data, no unneeded metadata (Linux only)')
            [CompletionResult]::new('--data', 'data', [CompletionResultType]::ParameterName, 'sync only file data, no unneeded metadata (Linux only)')
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
