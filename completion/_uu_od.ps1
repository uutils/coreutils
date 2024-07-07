
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_od' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_od'
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
        'uu_od' {
            [CompletionResult]::new('-A', 'A ', [CompletionResultType]::ParameterName, 'Select the base in which file offsets are printed.')
            [CompletionResult]::new('--address-radix', 'address-radix', [CompletionResultType]::ParameterName, 'Select the base in which file offsets are printed.')
            [CompletionResult]::new('-j', 'j', [CompletionResultType]::ParameterName, 'Skip bytes input bytes before formatting and writing.')
            [CompletionResult]::new('--skip-bytes', 'skip-bytes', [CompletionResultType]::ParameterName, 'Skip bytes input bytes before formatting and writing.')
            [CompletionResult]::new('-N', 'N ', [CompletionResultType]::ParameterName, 'limit dump to BYTES input bytes')
            [CompletionResult]::new('--read-bytes', 'read-bytes', [CompletionResultType]::ParameterName, 'limit dump to BYTES input bytes')
            [CompletionResult]::new('--endian', 'endian', [CompletionResultType]::ParameterName, 'byte order to use for multi-byte formats')
            [CompletionResult]::new('-S', 'S ', [CompletionResultType]::ParameterName, 'NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when BYTES is not specified.')
            [CompletionResult]::new('--strings', 'strings', [CompletionResultType]::ParameterName, 'NotImplemented: output strings of at least BYTES graphic chars. 3 is assumed when BYTES is not specified.')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 'select output format or formats')
            [CompletionResult]::new('--format', 'format', [CompletionResultType]::ParameterName, 'select output format or formats')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'output BYTES bytes per output line. 32 is implied when BYTES is not specified.')
            [CompletionResult]::new('--width', 'width', [CompletionResultType]::ParameterName, 'output BYTES bytes per output line. 32 is implied when BYTES is not specified.')
            [CompletionResult]::new('--help', 'help', [CompletionResultType]::ParameterName, 'Print help information.')
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'named characters, ignoring high-order bit')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'octal bytes')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'ASCII characters or backslash escapes')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'unsigned decimal 2-byte units')
            [CompletionResult]::new('-D', 'D ', [CompletionResultType]::ParameterName, 'unsigned decimal 4-byte units')
            [CompletionResult]::new('-o', 'o', [CompletionResultType]::ParameterName, 'octal 2-byte units')
            [CompletionResult]::new('-I', 'I ', [CompletionResultType]::ParameterName, 'decimal 8-byte units')
            [CompletionResult]::new('-L', 'L ', [CompletionResultType]::ParameterName, 'decimal 8-byte units')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'decimal 4-byte units')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'decimal 8-byte units')
            [CompletionResult]::new('-x', 'x', [CompletionResultType]::ParameterName, 'hexadecimal 2-byte units')
            [CompletionResult]::new('-h', 'h', [CompletionResultType]::ParameterName, 'hexadecimal 2-byte units')
            [CompletionResult]::new('-O', 'O ', [CompletionResultType]::ParameterName, 'octal 4-byte units')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'decimal 2-byte units')
            [CompletionResult]::new('-X', 'X ', [CompletionResultType]::ParameterName, 'hexadecimal 4-byte units')
            [CompletionResult]::new('-H', 'H ', [CompletionResultType]::ParameterName, 'hexadecimal 4-byte units')
            [CompletionResult]::new('-e', 'e', [CompletionResultType]::ParameterName, 'floating point double precision (64-bit) units')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'floating point double precision (32-bit) units')
            [CompletionResult]::new('-F', 'F ', [CompletionResultType]::ParameterName, 'floating point double precision (64-bit) units')
            [CompletionResult]::new('-v', 'v', [CompletionResultType]::ParameterName, 'do not use * to mark line suppression')
            [CompletionResult]::new('--output-duplicates', 'output-duplicates', [CompletionResultType]::ParameterName, 'do not use * to mark line suppression')
            [CompletionResult]::new('--traditional', 'traditional', [CompletionResultType]::ParameterName, 'compatibility mode with one input, offset and label.')
            [CompletionResult]::new('-V', 'V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', 'version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
