
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'uu_hostname' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'uu_hostname'
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
        'uu_hostname' {
            [CompletionResult]::new('-d', 'd', [CompletionResultType]::ParameterName, 'Display the name of the DNS domain if possible')
            [CompletionResult]::new('--domain', 'domain', [CompletionResultType]::ParameterName, 'Display the name of the DNS domain if possible')
            [CompletionResult]::new('-i', 'i', [CompletionResultType]::ParameterName, 'Display the network address(es) of the host')
            [CompletionResult]::new('--ip-address', 'ip-address', [CompletionResultType]::ParameterName, 'Display the network address(es) of the host')
            [CompletionResult]::new('-f', 'f', [CompletionResultType]::ParameterName, 'Display the FQDN (Fully Qualified Domain Name) (default)')
            [CompletionResult]::new('--fqdn', 'fqdn', [CompletionResultType]::ParameterName, 'Display the FQDN (Fully Qualified Domain Name) (default)')
            [CompletionResult]::new('-s', 's', [CompletionResultType]::ParameterName, 'Display the short hostname (the portion before the first dot) if possible')
            [CompletionResult]::new('--short', 'short', [CompletionResultType]::ParameterName, 'Display the short hostname (the portion before the first dot) if possible')
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
