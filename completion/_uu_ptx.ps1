
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_ptx' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_ptx'
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
        'uu_ptx' {
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'use STRING for flagging line truncations')
            [CompletionResult]::new('--flag-truncation', 'flag-truncation', [CompletionResultType]::ParameterName, 'use STRING for flagging line truncations')
            [CompletionResult]::new('-M', 'M ', [CompletionResultType]::ParameterName, 'macro name to use instead of ''xx''')
            [CompletionResult]::new('--macro-name', 'macro-name', [CompletionResultType]::ParameterName, 'macro name to use instead of ''xx''')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'for end of lines or end of sentences')
            [CompletionResult]::new('--sentence-regexp', 'sentence-regexp', [CompletionResultType]::ParameterName, 'for end of lines or end of sentences')
            [CompletionResult]::new('-W', 'W ', [CompletionResultType]::ParameterName, 'use REGEXP to match each keyword')
            [CompletionResult]::new('--word-regexp', 'word-regexp', [CompletionResultType]::ParameterName, 'use REGEXP to match each keyword')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'word break characters in this FILE')
            [CompletionResult]::new('--break-file', 'break-file', [CompletionResultType]::ParameterName, 'word break characters in this FILE')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'gap size in columns between output fields')
            [CompletionResult]::new('--gap-size', 'gap-size', [CompletionResultType]::ParameterName, 'gap size in columns between output fields')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'read ignore word list from FILE')
            [CompletionResult]::new('--ignore-file', 'ignore-file', [CompletionResultType]::ParameterName, 'read ignore word list from FILE')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'read only word list from this FILE')
            [CompletionResult]::new('--only-file', 'only-file', [CompletionResultType]::ParameterName, 'read only word list from this FILE')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'output width in columns, reference excluded')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'output width in columns, reference excluded')
            [CompletionResult]::new('-A', 'A ', [CompletionResultType]::ParameterName, 'output automatically generated references')
            [CompletionResult]::new('--auto-reference', 'auto-reference', [CompletionResultType]::ParameterName, 'output automatically generated references')
            [CompletionResult]::new('-G', 'G ', [CompletionResultType]::ParameterName, 'behave more like System V ''ptx''')
            [CompletionResult]::new('--traditional', 'traditional', [CompletionResultType]::ParameterName, 'behave more like System V ''ptx''')
            [CompletionResult]::new('-O', 'O ', [CompletionResultType]::ParameterName, 'generate output as roff directives')
            [CompletionResult]::new('--format=roff', 'format=roff', [CompletionResultType]::ParameterName, 'generate output as roff directives')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'put references at right, not counted in -w')
            [CompletionResult]::new('--right-side-refs', 'right-side-refs', [CompletionResultType]::ParameterName, 'put references at right, not counted in -w')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'generate output as TeX directives')
            [CompletionResult]::new('--format=tex', 'format=tex', [CompletionResultType]::ParameterName, 'generate output as TeX directives')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'fold lower case to upper case for sorting')
            [CompletionResult]::new('--ignore-case', 'ignore-case', [CompletionResultType]::ParameterName, 'fold lower case to upper case for sorting')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'first field of each line is a reference')
            [CompletionResult]::new('--references', 'references', [CompletionResultType]::ParameterName, 'first field of each line is a reference')
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
