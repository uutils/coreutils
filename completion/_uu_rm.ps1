
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_rm' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_rm'
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
        'uu_rm' {
            [CompletionResult]::new('--interactive', 'interactive', [CompletionResultType]::ParameterName, 'prompt according to WHEN: never, once (-I), or always (-i). Without WHEN, prompts always')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'ignore nonexistent files and arguments, never prompt')
            [CompletionResult]::new('--force', 'force', [CompletionResultType]::ParameterName, 'ignore nonexistent files and arguments, never prompt')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'prompt before every removal')
            [CompletionResult]::new('-I', 'I ', [CompletionResultType]::ParameterName, 'prompt once before removing more than three files, or when removing recursively. Less intrusive than -i, while still giving some protection against most mistakes')
            [CompletionResult]::new('--one-file-system', 'one-file-system', [CompletionResultType]::ParameterName, 'when removing a hierarchy recursively, skip any directory that is on a file system different from that of the corresponding command line argument (NOT IMPLEMENTED)')
            [CompletionResult]::new('--no-preserve-root', 'no-preserve-root', [CompletionResultType]::ParameterName, 'do not treat ''/'' specially')
            [CompletionResult]::new('--preserve-root', 'preserve-root', [CompletionResultType]::ParameterName, 'do not remove ''/'' (default)')
            [CompletionResult]::new('-r', 'r', [CompletionResultType]::ParameterName, 'remove directories and their contents recursively')
            [CompletionResult]::new('-R', 'R ', [CompletionResultType]::ParameterName, 'remove directories and their contents recursively')
            [CompletionResult]::new('--recursive', 'recursive', [CompletionResultType]::ParameterName, 'remove directories and their contents recursively')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'remove empty directories')
            [CompletionResult]::new('--dir', 'dir', [CompletionResultType]::ParameterName, 'remove empty directories')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('--verbose', 'verbose', [CompletionResultType]::ParameterName, 'explain what is being done')
            [CompletionResult]::new('--presume-input-tty', 'presume-input-tty', [CompletionResultType]::ParameterName, 'presume-input-tty')
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
