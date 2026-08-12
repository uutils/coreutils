# Chaînes communes partagées entre toutes les commandes uutils
# Principalement pour clap

# Mots génériques
common-error = erreur
common-tip = conseil
common-usage = Utilisation
common-help = aide
common-version = version
common-write-error = erreur d'écriture

# Messages d'erreur clap communs
clap-error-unexpected-argument = { $error_word } : argument inattendu '{ $arg }' trouvé
clap-error-unexpected-argument-simple = argument inattendu
clap-error-similar-argument = { $tip_word } : un argument similaire existe : '{ $suggestion }'
clap-error-pass-as-value = { $tip_word } : pour passer '{ $arg }' comme valeur, utilisez '{ $tip_command }'
clap-error-invalid-value = { $error_word } : valeur invalide '{ $value }' pour '{ $option }'
clap-error-value-required = { $error_word } : une valeur est requise pour '{ $option }' mais aucune n'a été fournie
clap-error-missing-required-arguments = { $error_word } : les arguments requis suivants n'ont pas été fournis :
clap-error-possible-values = valeurs possibles
clap-error-help-suggestion = Pour plus d'informations, essayez '{ $command } --help'.
common-help-suggestion = Pour plus d'informations, essayez '--help'.

# Modèles de texte d'aide communs
help-flag-help = Afficher les informations d'aide
help-flag-version = Afficher les informations de version

# Contextes d'erreur communs
error-io = Erreur E/S
error-permission-denied = Permission refusée
error-file-not-found = Aucun fichier ou répertoire de ce type
error-no-such-process = Aucun processus de ce type
error-invalid-argument = Argument invalide
error-is-a-directory = { $file }: Est un répertoire

# Actions communes
action-copying = copie
action-moving = déplacement
action-removing = suppression
action-creating = création
action-reading = lecture
action-writing = écriture

# Messages d'erreur SELinux
selinux-error-not-enabled = SELinux n'est pas activé sur ce système
selinux-error-file-open-failure = échec de l'ouverture du fichier : { $error }
selinux-error-context-retrieval-failure = échec de la récupération du contexte de sécurité : { $error }
selinux-error-context-set-failure = échec de la définition du contexte de création de fichier par défaut à '{ $context }' : { $error }
selinux-error-context-conversion-failure = échec de la définition du contexte de création de fichier par défaut à '{ $context }' : { $error }
selinux-error-operation-not-supported = opération non prise en charge

# Messages d'erreur de traversée sécurisée
safe-traversal-error-path-contains-null = le chemin contient un octet null
safe-traversal-error-open-failed = échec de l'ouverture de { $path } : { $source }
safe-traversal-error-stat-failed = échec de l'analyse de { $path } : { $source }
safe-traversal-error-read-dir-failed = échec de la lecture du répertoire { $path } : { $source }
safe-traversal-error-unlink-failed = échec de la suppression de { $path } : { $source }
safe-traversal-error-invalid-fd = descripteur de fichier invalide
safe-traversal-current-directory = <répertoire courant>
safe-traversal-directory = <répertoire>

# Messages relatifs au module checksum
checksum-no-properly-formatted = { $checksum_file }: aucune ligne correctement formattée n'a été trouvée
checksum-no-file-verified = { $checksum_file }: aucun fichier n'a été vérifié
checksum-error-failed-to-read-input = échec de la lecture de l'entrée
checksum-bad-format = { $count ->
    [1] { $count } ligne invalide
   *[other] { $count } lignes invalides
}
checksum-failed-cksum = { $count ->
    [1] { $count } somme de hachage ne correspond PAS
   *[other] { $count } sommes de hachage ne correspondent PAS
}
checksum-failed-open-file = { $count ->
    [1] { $count } fichier passé n'a pas pu être lu
   *[other] { $count } fichiers passés n'ont pas pu être lu
}
checksum-error-algo-bad-format = { $file }: { $line }: ligne invalide pour { $algo }

# Messages uudoc pour les exemples tldr
uudoc-tldr-attribution = Les exemples sont fournis par le [projet tldr-pages](https://tldr.sh) sous la [licence CC BY 4.0](https://github.com/tldr-pages/tldr/blob/main/LICENSE.md).
uudoc-tldr-disclaimer = Veuillez noter que, uutils étant en cours de développement, certains exemples peuvent échouer.

# Messages d'analyse des modes symboliques
mode-error-unexpected-end = fin de mode inattendue
mode-error-invalid-operator = opérateur invalide (+, - ou = attendu, mais { $operator } trouvé)

# Étiquettes de diagnostic : ce que le caret désigne dans un mode
mode-diag-label-missing-operator = cette clause indique qui, mais pas quoi changer
mode-diag-label-invalid-number = n'est pas un mode octal
mode-diag-help-syntax = un mode est soit octal, comme 644, soit des clauses comme u+rwx,go-w
# Messages d'analyse des chaînes de format (printf, seq, env, ...)
format-error-invalid-spec = %{ $spec } : spécification de conversion invalide
format-error-too-many-specs = le format '{ $format }' a trop de directives %
format-error-no-spec = le format '{ $format }' n'a pas de directive %
format-error-ends-with-percent = le format { $format } se termine par %
format-error-invalid-precision = précision invalide : '{ $precision }'
format-error-wrong-spec-type = type de directive % incorrect
format-error-write = erreur d'écriture : { $error }
format-error-no-more-arguments = plus d'arguments
format-error-invalid-argument = argument invalide
format-error-missing-hex = nombre hexadécimal manquant dans l'échappement
format-error-invalid-universal-character = nom de caractère universel invalide \{ $escape }{ $digits }

# Le mot en tête de la ligne de conseil d'un rapport avec caret
diagnostics-help-label = Aide{" "}

# Erreurs de somme de contrôle (cksum, md5sum, sha*sum, b2sum)
checksum-error-raw-multiple-files = l'option --raw n'est pas prise en charge avec plusieurs fichiers
checksum-error-check-only-flag = l'option --{ $flag } n'a de sens que lors de la vérification de sommes de contrôle
checksum-error-length-required = --length est requis pour { $algorithm }
checksum-error-invalid-length = longueur invalide : { $length }
checksum-error-length-too-big-for-blake = la longueur maximale d'empreinte pour { $algorithm } est de 512 bits
checksum-error-length-not-multiple-of-8 = la longueur n'est pas un multiple de 8
checksum-error-invalid-length-for-sha = la longueur d'empreinte pour { $algorithm } doit être 224, 256, 384 ou 512
checksum-error-length-required-for-sha = --algorithm={ $algorithm } nécessite de préciser --length 224, 256, 384 ou 512
checksum-error-length-only-for-blake2b-sha2-sha3 = --length n'est pris en charge qu'avec --algorithm blake2b, sha2 ou sha3
checksum-error-binary-text-conflict = les options --binary et --text n'ont pas de sens lors de la vérification de sommes de contrôle
checksum-error-text-without-untagged = le mode --text n'est pris en charge qu'avec --untagged
checksum-error-tag-check = l'option --tag n'a pas de sens lors de la vérification de sommes de contrôle
checksum-error-text-after-tag = --tag ne prend pas en charge le mode --text
checksum-error-algorithm-not-supported-with-check = --check n'est pas pris en charge avec --algorithm={"{"}bsd,sysv,crc,crc32b{"}"}
checksum-error-combine-multiple-algorithms = Vous ne pouvez pas combiner plusieurs algorithmes de hachage !
checksum-error-need-algorithm-to-hash = Un algorithme de hachage est nécessaire.
    Utilisez --help pour plus d'informations.
checksum-error-unknown-algorithm = algorithme inconnu : { $algorithm } : clap aurait dû empêcher ce cas
