
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_du' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_du'
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
        'uu_du' {
            [CompletionResult]::new('-B', 'B ', [CompletionResultType]::ParameterName, 'scale sizes by SIZE before printing them. E.g., ''-BM'' prints sizes in units of 1,048,576 bytes. See SIZE format below.')
            [CompletionResult]::new('--block-size', 'block-size', [CompletionResultType]::ParameterName, 'scale sizes by SIZE before printing them. E.g., ''-BM'' prints sizes in units of 1,048,576 bytes. See SIZE format below.')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize')
            [CompletionResult]::new('--max-depth', 'max-depth', [CompletionResultType]::ParameterName, 'print the total for a directory (or file, with --all) only if it is N or fewer levels below the command line argument;  --max-depth=0 is the same as --summarize')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative')
            [CompletionResult]::new('--threshold', 'threshold', [CompletionResultType]::ParameterName, 'exclude entries smaller than SIZE if positive, or entries greater than SIZE if negative')
            [CompletionResult]::new('--exclude', 'exclude', [CompletionResultType]::ParameterName, 'exclude files that match PATTERN')
            [CompletionResult]::new('-X', 'X ', [CompletionResultType]::ParameterName, 'exclude files that match any pattern in FILE')
            [CompletionResult]::new('--exclude-from', 'exclude-from', [CompletionResultType]::ParameterName, 'exclude files that match any pattern in FILE')
            [CompletionResult]::new('--files0-from', 'files0-from', [CompletionResultType]::ParameterName, 'summarize device usage of the NUL-terminated file names specified in file F; if F is -, then read names from standard input')
            [CompletionResult]::new('--time', 'time', [CompletionResultType]::ParameterName, 'show time of the last modification of any file in the directory, or any of its subdirectories. If WORD is given, show time as WORD instead of modification time: atime, access, use, ctime, status, birth or creation')
            [CompletionResult]::new('--time-style', 'time-style', [CompletionResultType]::ParameterName, 'show times using style STYLE: full-iso, long-iso, iso, +FORMAT FORMAT is interpreted like ''date''')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'write counts for all files, not just directories')
            [CompletionResult]::new('--all', 'all', [CompletionResultType]::ParameterName, 'write counts for all files, not just directories')
            [CompletionResult]::new('--apparent-size', 'apparent-size', [CompletionResultType]::ParameterName, 'print apparent sizes, rather than disk usage although the apparent size is usually smaller, it may be larger due to holes in (''sparse'') files, internal fragmentation, indirect blocks, and the like')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'equivalent to ''--apparent-size --block-size=1''')
            [CompletionResult]::new('--bytes', 'bytes', [CompletionResultType]::ParameterName, 'equivalent to ''--apparent-size --block-size=1''')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'produce a grand total')
            [CompletionResult]::new('--total', 'total', [CompletionResultType]::ParameterName, 'produce a grand total')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'print sizes in human readable format (e.g., 1K 234M 2G)')
            [CompletionResult]::new('--human-readable', 'human-readable', [CompletionResultType]::ParameterName, 'print sizes in human readable format (e.g., 1K 234M 2G)')
            [CompletionResult]::new('--inodes', 'inodes', [CompletionResultType]::ParameterName, 'list inode usage information instead of block usage like --block-size=1K')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'like --block-size=1K')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'count sizes many times if hard linked')
            [CompletionResult]::new('--count-links', 'count-links', [CompletionResultType]::ParameterName, 'count sizes many times if hard linked')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'follow all symbolic links')
            [CompletionResult]::new('--dereference', 'dereference', [CompletionResultType]::ParameterName, 'follow all symbolic links')
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'follow only symlinks that are listed on the command line')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'follow only symlinks that are listed on the command line')
            [CompletionResult]::new('--dereference-args', 'dereference-args', [CompletionResultType]::ParameterName, 'follow only symlinks that are listed on the command line')
            [CompletionResult]::new('-P', 'P ', [CompletionResultType]::ParameterName, 'don''t follow any symbolic links (this is the default)')
            [CompletionResult]::new('--no-dereference', 'no-dereference', [CompletionResultType]::ParameterName, 'don''t follow any symbolic links (this is the default)')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'like --block-size=1M')
            [CompletionResult]::new('-0', '0', [CompletionResultType]::ParameterName, 'end each output line with 0 byte rather than newline')
            [CompletionResult]::new('--null', 'null', [CompletionResultType]::ParameterName, 'end each output line with 0 byte rather than newline')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'do not include size of subdirectories')
            [CompletionResult]::new('--separate-dirs', 'separate-dirs', [CompletionResultType]::ParameterName, 'do not include size of subdirectories')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'display only a total for each argument')
            [CompletionResult]::new('--summarize', 'summarize', [CompletionResultType]::ParameterName, 'display only a total for each argument')
            [CompletionResult]::new('--si', 'si', [CompletionResultType]::ParameterName, 'like -h, but use powers of 1000 not 1024')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'skip directories on different file systems')
            [CompletionResult]::new('--one-file-system', 'one-file-system', [CompletionResultType]::ParameterName, 'skip directories on different file systems')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'verbose mode (option not present in GNU/Coreutils)')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'verbose mode (option not present in GNU/Coreutils)')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
