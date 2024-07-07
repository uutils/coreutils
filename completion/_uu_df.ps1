
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_df' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_df'
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
        'uu_df' {
            [CompletionResult]::new('-B', 'B ', [CompletionResultType]::ParameterName, 'scale sizes by SIZE before printing them; e.g.''-BM'' prints sizes in units of 1,048,576 bytes')
            [CompletionResult]::new('--block-size', 'block-size', [CompletionResultType]::ParameterName, 'scale sizes by SIZE before printing them; e.g.''-BM'' prints sizes in units of 1,048,576 bytes')
            [CompletionResult]::new('--output', 'output', [CompletionResultType]::ParameterName, 'use the output format defined by FIELD_LIST, or print all fields if FIELD_LIST is omitted.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'limit listing to file systems of type TYPE')
            [CompletionResult]::new('--type', 'type', [CompletionResultType]::ParameterName, 'limit listing to file systems of type TYPE')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'limit listing to file systems not of type TYPE')
            [CompletionResult]::new('--exclude-type', 'exclude-type', [CompletionResultType]::ParameterName, 'limit listing to file systems not of type TYPE')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'include dummy file systems')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'include dummy file systems')
            [CompletionResult]::new('--total', 'total', [CompletionResultType]::ParameterName, 'produce a grand total')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'print sizes in human readable format (e.g., 1K 234M 2G)')
            [CompletionResult]::new('--human-readable', 'human-readable', [CompletionResultType]::ParameterName, 'print sizes in human readable format (e.g., 1K 234M 2G)')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'likewise, but use powers of 1000 not 1024')
            [CompletionResult]::new('--si', 'si', [CompletionResultType]::ParameterName, 'likewise, but use powers of 1000 not 1024')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'list inode information instead of block usage')
            [CompletionResult]::new('--inodes', 'inodes', [CompletionResultType]::ParameterName, 'list inode information instead of block usage')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'like --block-size=1K')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'limit listing to local file systems')
            [CompletionResult]::new('--local', 'local', [CompletionResultType]::ParameterName, 'limit listing to local file systems')
            [CompletionResult]::new('--no-sync', 'no-sync', [CompletionResultType]::ParameterName, 'do not invoke sync before getting usage info (default)')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'use the POSIX output format')
            [CompletionResult]::new('--portability', 'portability', [CompletionResultType]::ParameterName, 'use the POSIX output format')
            [CompletionResult]::new('--sync', 'sync', [CompletionResultType]::ParameterName, 'invoke sync before getting usage info (non-windows only)')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'print file system type')
            [CompletionResult]::new('--print-type', 'print-type', [CompletionResultType]::ParameterName, 'print file system type')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
