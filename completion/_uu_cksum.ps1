
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_cksum' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_cksum'
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
        'uu_cksum' {
            [CompletionResult]::new('-a', 'a', [CompletionResultType]::ParameterName, 'select the digest type to use. See DIGEST below')
            [CompletionResult]::new('--algorithm', 'algorithm', [CompletionResultType]::ParameterName, 'select the digest type to use. See DIGEST below')
            [CompletionResult]::new('-l', 'l', [CompletionResultType]::ParameterName, 'digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8')
            [CompletionResult]::new('--length', 'length', [CompletionResultType]::ParameterName, 'digest length in bits; must not exceed the max for the blake2 algorithm and must be a multiple of 8')
            [CompletionResult]::new('--untagged', 'untagged', [CompletionResultType]::ParameterName, 'create a reversed style checksum, without digest type')
            [CompletionResult]::new('--tag', 'tag', [CompletionResultType]::ParameterName, 'create a BSD style checksum, undo --untagged (default)')
            [CompletionResult]::new('--raw', 'raw', [CompletionResultType]::ParameterName, 'emit a raw binary digest, not hexadecimal')
            [CompletionResult]::new('--strict', 'strict', [CompletionResultType]::ParameterName, 'exit non-zero for improperly formatted checksum lines')
            [CompletionResult]::new('-c', 'c', [CompletionResultType]::ParameterName, 'read hashsums from the FILEs and check them')
            [CompletionResult]::new('--check', 'check', [CompletionResultType]::ParameterName, 'read hashsums from the FILEs and check them')
            [CompletionResult]::new('--base64', 'base64', [CompletionResultType]::ParameterName, 'emit a base64 digest, not hexadecimal')
            [CompletionResult]::new('-t', 't', [CompletionResultType]::ParameterName, 't')
            [CompletionResult]::new('--text', 'text', [CompletionResultType]::ParameterName, 'text')
            [CompletionResult]::new('-b', 'b', [CompletionResultType]::ParameterName, 'b')
            [CompletionResult]::new('--binary', 'binary', [CompletionResultType]::ParameterName, 'binary')
            [CompletionResult]::new('-w', 'w', [CompletionResultType]::ParameterName, 'warn about improperly formatted checksum lines')
            [CompletionResult]::new('--warn', 'warn', [CompletionResultType]::ParameterName, 'warn about improperly formatted checksum lines')
            [CompletionResult]::new('--status', 'status', [CompletionResultType]::ParameterName, 'don''t output anything, status code shows success')
            [CompletionResult]::new('--quiet', 'quiet', [CompletionResultType]::ParameterName, 'don''t print OK for each successfully verified file')
            [CompletionResult]::new('--ignore-missing', 'ignore-missing', [CompletionResultType]::ParameterName, 'don''t fail or report status for missing files')
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
