
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_split' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_split'
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
        'uu_split' {
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'put SIZE bytes per output file')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'put SIZE bytes per output file')
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'put at most SIZE bytes of lines per output file')
            [CompletionResult]::new('--line-bytes', 'line-bytes', [CompletionResultType]::ParameterName, 'put at most SIZE bytes of lines per output file')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'put NUMBER lines/records per output file')
            [CompletionResult]::new('--lines', 'lines', [CompletionResultType]::ParameterName, 'put NUMBER lines/records per output file')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'generate CHUNKS output files; see explanation below')
            [CompletionResult]::new('--number', 'number', [CompletionResultType]::ParameterName, 'generate CHUNKS output files; see explanation below')
            [CompletionResult]::new('--additional-suffix', 'additional-suffix', [CompletionResultType]::ParameterName, 'additional SUFFIX to append to output file names')
            [CompletionResult]::new('--filter', 'filter', [CompletionResultType]::ParameterName, 'write to shell COMMAND; file name is $FILE (Currently not implemented for Windows)')
            [CompletionResult]::new('--numeric-suffixes', 'numeric-suffixes', [CompletionResultType]::ParameterName, 'same as -d, but allow setting the start value')
            [CompletionResult]::new('--hex-suffixes', 'hex-suffixes', [CompletionResultType]::ParameterName, 'same as -x, but allow setting the start value')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'generate suffixes of length N (default 2)')
            [CompletionResult]::new('--suffix-length', 'suffix-length', [CompletionResultType]::ParameterName, 'generate suffixes of length N (default 2)')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'use SEP instead of newline as the record separator; ''\0'' (zero) specifies the NUL character')
            [CompletionResult]::new('--separator', 'separator', [CompletionResultType]::ParameterName, 'use SEP instead of newline as the record separator; ''\0'' (zero) specifies the NUL character')
            [CompletionResult]::new('--io-blksize', 'io-blksize', [CompletionResultType]::ParameterName, 'io-blksize')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'do not generate empty output files with ''-n''')
            [CompletionResult]::new('--elide-empty-files', 'elide-empty-files', [CompletionResultType]::ParameterName, 'do not generate empty output files with ''-n''')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'use numeric suffixes starting at 0, not alphabetic')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'use hex suffixes starting at 0, not alphabetic')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'print a diagnostic just before each output file is opened')
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
