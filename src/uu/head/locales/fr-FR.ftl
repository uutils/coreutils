head-about = Affiche les 10 premières lignes de chaque FICHIER sur la sortie standard.
  Avec plus d'un FICHIER, précède chacun d'un en-tête donnant le nom du fichier.
  Sans FICHIER, ou quand FICHIER est -, lit l'entrée standard.

  Les arguments obligatoires pour les drapeaux longs sont obligatoires pour les drapeaux courts aussi.
head-usage = head [DRAPEAU]... [FICHIER]...

# Messages d'aide
head-help-bytes = affiche les premiers NUM octets de chaque fichier ;
 avec le préfixe '-', affiche tout sauf les derniers
 NUM octets de chaque fichier
head-help-lines = affiche les premières NUM lignes au lieu des 10 premières ;
 avec le préfixe '-', affiche tout sauf les dernières
 NUM lignes de chaque fichier
head-help-quiet = n'affiche jamais les en-têtes donnant les noms de fichiers
head-help-verbose = affiche toujours les en-têtes donnant les noms de fichiers
head-help-zero-terminated = le délimiteur de ligne est NUL, pas nouvelle ligne

# Messages d'erreur
head-error-reading-file = erreur lors de la lecture de {$name} : {$err}
head-error-parse-error = erreur d'analyse : {$err}
head-error-bad-encoding = mauvais encodage d'argument
head-error-num-too-large = le nombre d'octets ou de lignes est trop grand
head-error-clap = erreur clap : {$err}
head-error-invalid-bytes = nombre d'octets invalide : {$err}
head-error-invalid-lines = nombre de lignes invalide : {$err}
head-error-bad-argument-format = format d'argument incorrect : {$arg}
head-error-writing-stdout = erreur lors de l'écriture sur 'sortie standard' : {$err}
head-error-cannot-open = impossible d'ouvrir {$name} en lecture

# En-têtes de sortie
head-header-stdin = ==> entrée standard <==
