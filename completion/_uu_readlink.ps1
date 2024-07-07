
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_readlink' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_readlink'
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
        'uu_readlink' {
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively; all but the last component must exist')
            [CompletionResult]::new('--canonicalize', 'canonicalize', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively; all but the last component must exist')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, all components must exist')
            [CompletionResult]::new('--canonicalize-existing', 'canonicalize-existing', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, all components must exist')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence')
            [CompletionResult]::new('--canonicalize-missing', 'canonicalize-missing', [CompletionResultType]::ParameterName, 'canonicalize by following every symlink in every component of the given name recursively, without requirements on components existence')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'do not output the trailing delimiter')
            [CompletionResult]::new('--no-newline', 'no-newline', [CompletionResultType]::ParameterName, 'do not output the trailing delimiter')
            [CompletionResult]::new('-q', 'q', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'suppress most error messages')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'report error message')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'report error message')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'separate output with NUL rather than newline')
            [CompletionResult]::new('--zero', 'zero', [CompletionResultType]::ParameterName, 'separate output with NUL rather than newline')
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
