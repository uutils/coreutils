
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_nl' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_nl'
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
        'uu_nl' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'use STYLE for numbering body lines')
            [CompletionResult]::new('--body-numbering', 'body-numbering', [CompletionResultType]::ParameterName, 'use STYLE for numbering body lines')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'use CC for separating logical pages')
            [CompletionResult]::new('--section-delimiter', 'section-delimiter', [CompletionResultType]::ParameterName, 'use CC for separating logical pages')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'use STYLE for numbering footer lines')
            [CompletionResult]::new('--footer-numbering', 'footer-numbering', [CompletionResultType]::ParameterName, 'use STYLE for numbering footer lines')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'use STYLE for numbering header lines')
            [CompletionResult]::new('--header-numbering', 'header-numbering', [CompletionResultType]::ParameterName, 'use STYLE for numbering header lines')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'line number increment at each line')
            [CompletionResult]::new('--line-increment', 'line-increment', [CompletionResultType]::ParameterName, 'line number increment at each line')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'group of NUMBER empty lines counted as one')
            [CompletionResult]::new('--join-blank-lines', 'join-blank-lines', [CompletionResultType]::ParameterName, 'group of NUMBER empty lines counted as one')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'insert line numbers according to FORMAT')
            [CompletionResult]::new('--number-format', 'number-format', [CompletionResultType]::ParameterName, 'insert line numbers according to FORMAT')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'add STRING after (possible) line number')
            [CompletionResult]::new('--number-separator', 'number-separator', [CompletionResultType]::ParameterName, 'add STRING after (possible) line number')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'first line number on each logical page')
            [CompletionResult]::new('--starting-line-number', 'starting-line-number', [CompletionResultType]::ParameterName, 'first line number on each logical page')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'use NUMBER columns for line numbers')
            [CompletionResult]::new('--number-width', 'number-width', [CompletionResultType]::ParameterName, 'use NUMBER columns for line numbers')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-p', 'p', [CompletionResultType]::ParameterName, 'do not reset line numbers at logical pages')
            [CompletionResult]::new('--no-renumber', 'no-renumber', [CompletionResultType]::ParameterName, 'do not reset line numbers at logical pages')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
