
use builtin;
use str;

set edit:completion:arg-completer[uu_hostname] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'uu_hostname'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'uu_hostname'= {
            cand -d 'Display the name of the DNS domain if possible'
            cand --domain 'Display the name of the DNS domain if possible'
            cand -i 'Display the network address(es) of the host'
            cand --ip-address 'Display the network address(es) of the host'
            cand -f 'Display the FQDN (Fully Qualified Domain Name) (default)'
            cand --fqdn 'Display the FQDN (Fully Qualified Domain Name) (default)'
            cand -s 'Display the short hostname (the portion before the first dot) if possible'
            cand --short 'Display the short hostname (the portion before the first dot) if possible'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
