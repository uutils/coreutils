# Chaînes communes partagées entre toutes les commandes uutils
# Principalement pour clap

# Mots génériques
common-error = erreur
common-tip = conseil
common-usage = Utilisation
common-help = aide
common-version = version

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
