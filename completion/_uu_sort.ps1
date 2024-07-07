
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_sort' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_sort'
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
        'uu_sort' {
            [CompletionResult]::new('--sort', 'sort', [CompletionResultType]::ParameterName, 'sort')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'check for sorted input; do not sort')
            [CompletionResult]::new('--check', 'check', [CompletionResultType]::ParameterName, 'check for sorted input; do not sort')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'write output to FILENAME instead of stdout')
            [CompletionResult]::new('--output', 'output', [CompletionResultType]::ParameterName, 'write output to FILENAME instead of stdout')
            [CompletionResult]::new('-k', 'k', [CompletionResultType]::ParameterName, 'sort by a key')
            [CompletionResult]::new('--key', 'key', [CompletionResultType]::ParameterName, 'sort by a key')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'custom separator for -k')
            [CompletionResult]::new('--field-separator', 'field-separator', [CompletionResultType]::ParameterName, 'custom separator for -k')
            [CompletionResult]::new('--parallel', 'parallel', [CompletionResultType]::ParameterName, 'change the number of threads running concurrently to NUM_THREADS')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'sets the maximum SIZE of each segment in number of sorted items')
            [CompletionResult]::new('--buffer-size', 'buffer-size', [CompletionResultType]::ParameterName, 'sets the maximum SIZE of each segment in number of sorted items')
            [CompletionResult]::new('-T', 'T ', [CompletionResultType]::ParameterName, 'use DIR for temporaries, not $TMPDIR or /tmp')
            [CompletionResult]::new('--temporary-directory', 'temporary-directory', [CompletionResultType]::ParameterName, 'use DIR for temporaries, not $TMPDIR or /tmp')
            [CompletionResult]::new('--compress-program', 'compress-program', [CompletionResultType]::ParameterName, 'compress temporary files with PROG, decompress with PROG -d; PROG has to take input from stdin and output to stdout')
            [CompletionResult]::new('--batch-size', 'batch-size', [CompletionResultType]::ParameterName, 'Merge at most N_MERGE inputs at once.')
            [CompletionResult]::new('--files0-from', 'files0-from', [CompletionResultType]::ParameterName, 'read input from the files specified by NUL-terminated NUL_FILES')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version information.')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'compare according to human readable sizes, eg 1M > 100k')
            [CompletionResult]::new('--human-numeric-sort', 'human-numeric-sort', [CompletionResultType]::ParameterName, 'compare according to human readable sizes, eg 1M > 100k')
            [CompletionResult]::new('-M', 'M ', [CompletionResultType]::ParameterName, 'compare according to month name abbreviation')
            [CompletionResult]::new('--month-sort', 'month-sort', [CompletionResultType]::ParameterName, 'compare according to month name abbreviation')
            [CompletionResult]::new('-n', 'n', [CompletionResultType]::ParameterName, 'compare according to string numerical value')
            [CompletionResult]::new('--numeric-sort', 'numeric-sort', [CompletionResultType]::ParameterName, 'compare according to string numerical value')
            [CompletionResult]::new('-g', 'g', [CompletionResultType]::ParameterName, 'compare according to string general numerical value')
            [CompletionResult]::new('--general-numeric-sort', 'general-numeric-sort', [CompletionResultType]::ParameterName, 'compare according to string general numerical value')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Sort by SemVer version number, eg 1.12.2 > 1.1.2')
            [CompletionResult]::new('--version-sort', 'version-sort', [CompletionResultType]::ParameterName, 'Sort by SemVer version number, eg 1.12.2 > 1.1.2')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'shuffle in random order')
            [CompletionResult]::new('--random-sort', 'random-sort', [CompletionResultType]::ParameterName, 'shuffle in random order')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'consider only blanks and alphanumeric characters')
            [CompletionResult]::new('--dictionary-order', 'dictionary-order', [CompletionResultType]::ParameterName, 'consider only blanks and alphanumeric characters')
            [CompletionResult]::new('-m', 'm', [CompletionResultType]::ParameterName, 'merge already sorted files; do not sort')
            [CompletionResult]::new('--merge', 'merge', [CompletionResultType]::ParameterName, 'merge already sorted files; do not sort')
            [CompletionResult]::new('-C', 'C ', [CompletionResultType]::ParameterName, 'exit successfully if the given file is already sorted, and exit with status 1 otherwise.')
            [CompletionResult]::new('--check-silent', 'check-silent', [CompletionResultType]::ParameterName, 'exit successfully if the given file is already sorted, and exit with status 1 otherwise.')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'fold lower case to upper case characters')
            [CompletionResult]::new('--ignore-case', 'ignore-case', [CompletionResultType]::ParameterName, 'fold lower case to upper case characters')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'ignore nonprinting characters')
            [CompletionResult]::new('--ignore-nonprinting', 'ignore-nonprinting', [CompletionResultType]::ParameterName, 'ignore nonprinting characters')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'ignore leading blanks when finding sort keys in each line')
            [CompletionResult]::new('--ignore-leading-blanks', 'ignore-leading-blanks', [CompletionResultType]::ParameterName, 'ignore leading blanks when finding sort keys in each line')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'reverse the output')
            [CompletionResult]::new('--reverse', 'reverse', [CompletionResultType]::ParameterName, 'reverse the output')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'stabilize sort by disabling last-resort comparison')
            [CompletionResult]::new('--stable', 'stable', [CompletionResultType]::ParameterName, 'stabilize sort by disabling last-resort comparison')
            [CompletionResult]::new('-u', 'u', [CompletionResultType]::ParameterName, 'output only the first of an equal run')
            [CompletionResult]::new('--unique', 'unique', [CompletionResultType]::ParameterName, 'output only the first of an equal run')
            [CompletionResult]::new('-z', 'z', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
            [CompletionResult]::new('--zero-terminated', 'zero-terminated', [CompletionResultType]::ParameterName, 'line delimiter is NUL, not newline')
            [CompletionResult]::new('--debug', 'debug', [CompletionResultType]::ParameterName, 'underline the parts of the line that are actually used for sorting')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
