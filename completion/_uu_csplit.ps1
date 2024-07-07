
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_csplit' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_csplit'
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
        'uu_csplit' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'use sprintf FORMAT instead of %02d')
            [CompletionResult]::new('--suffix-format', 'suffix-format', [CompletionResultType]::ParameterName, 'use sprintf FORMAT instead of %02d')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'use PREFIX instead of ''xx''')
            [CompletionResult]::new('--prefix', 'prefix', [CompletionResultType]::ParameterName, 'use PREFIX instead of ''xx''')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'use specified number of digits instead of 2')
            [CompletionResult]::new('--digits', 'digits', [CompletionResultType]::ParameterName, 'use specified number of digits instead of 2')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'do not remove output files on errors')
            [CompletionResult]::new('--keep-files', 'keep-files', [CompletionResultType]::ParameterName, 'do not remove output files on errors')
            [CompletionResult]::new('--suppress-matched', 'suppress-matched', [CompletionResultType]::ParameterName, 'suppress the lines matching PATTERN')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'do not print counts of output file sizes')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'do not print counts of output file sizes')
            [CompletionResult]::new('--silent', 'silent', [CompletionResultType]::ParameterName, 'do not print counts of output file sizes')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'remove empty output files')
            [CompletionResult]::new('--elide-empty-files', 'elide-empty-files', [CompletionResultType]::ParameterName, 'remove empty output files')
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
