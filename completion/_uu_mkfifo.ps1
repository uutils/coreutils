
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_mkfifo' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_mkfifo'
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
        'uu_mkfifo' {
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'file permissions for the fifo')
            [CompletionResult]::new('--mode', 'mode', [CompletionResultType]::ParameterName, 'file permissions for the fifo')
            [CompletionResult]::new('--context', 'context', [CompletionResultType]::ParameterName, 'like -Z, or if CTX is specified then set the SELinux or SMACK security context to CTX')
            [CompletionResult]::new('-Z', 'Z ', [CompletionResultType]::ParameterName, 'set the SELinux security context to default type')
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
