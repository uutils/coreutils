
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_basenc' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_basenc'
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
        'uu_basenc' {
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)')
            [CompletionResult]::new('--wrap', 'wrap', [CompletionResultType]::ParameterName, 'wrap encoded lines after COLS character (default 76, 0 to disable wrapping)')
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'decode data')
            [CompletionResult]::new('--decode', 'decode', [CompletionResultType]::ParameterName, 'decode data')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'when decoding, ignore non-alphabetic characters')
            [CompletionResult]::new('--ignore-garbage', 'ignore-garbage', [CompletionResultType]::ParameterName, 'when decoding, ignore non-alphabetic characters')
            [CompletionResult]::new('--base64', 'base64', [CompletionResultType]::ParameterName, 'same as ''base64'' program')
            [CompletionResult]::new('--base64url', 'base64url', [CompletionResultType]::ParameterName, 'file- and url-safe base64')
            [CompletionResult]::new('--base32', 'base32', [CompletionResultType]::ParameterName, 'same as ''base32'' program')
            [CompletionResult]::new('--base32hex', 'base32hex', [CompletionResultType]::ParameterName, 'extended hex alphabet base32')
            [CompletionResult]::new('--base16', 'base16', [CompletionResultType]::ParameterName, 'hex encoding')
            [CompletionResult]::new('--base2lsbf', 'base2lsbf', [CompletionResultType]::ParameterName, 'bit string with least significant bit (lsb) first')
            [CompletionResult]::new('--base2msbf', 'base2msbf', [CompletionResultType]::ParameterName, 'bit string with most significant bit (msb) first')
            [CompletionResult]::new('--z85', 'z85', [CompletionResultType]::ParameterName, 'ascii85-like encoding;
when encoding, input length must be a multiple of 4;
when decoding, input length must be a multiple of 5')
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
