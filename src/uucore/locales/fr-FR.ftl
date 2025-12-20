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
safe-traversal-error-open-failed = échec de l'ouverture de '{ $path }' : { $source }
safe-traversal-error-stat-failed = échec de l'analyse de '{ $path }' : { $source }
safe-traversal-error-read-dir-failed = échec de la lecture du répertoire '{ $path }' : { $source }
safe-traversal-error-unlink-failed = échec de la suppression de '{ $path }' : { $source }
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

# Messages d'aide d'arguments checksum
checksum-help-algorithm = sélectionner le type de condensé à utiliser. Voir DIGEST ci-dessous
checksum-help-untagged = créer une somme de contrôle de style inversé, sans type de condensé
checksum-help-tag-default = créer une somme de contrôle de style BSD (par défaut)
checksum-help-tag = créer une somme de contrôle de style BSD
checksum-help-text = lire en mode texte (par défaut)
checksum-help-length = longueur du condensé en bits ; ne doit pas dépasser le maximum pour l'algorithme blake2 et doit être un multiple de 8
checksum-help-raw = émettre un condensé binaire brut, pas hexadécimal
checksum-help-strict = sortir avec un code non-zéro pour les lignes de somme de contrôle mal formatées
checksum-help-check = lire les sommes de hachage des FICHIERs et les vérifier
checksum-help-base64 = émettre un condensé base64, pas hexadécimal
checksum-help-warn = avertir des lignes de somme de contrôle mal formatées
checksum-help-status = ne rien afficher, le code de statut indique le succès
checksum-help-quiet = ne pas afficher OK pour chaque fichier vérifié avec succès
checksum-help-ignore-missing = ne pas échouer ou signaler le statut pour les fichiers manquants
checksum-help-zero = terminer chaque ligne de sortie avec NUL, pas un saut de ligne, et désactiver l'échappement des noms de fichiers
checksum-help-debug = afficher les informations de débogage sur la détection de la prise en charge matérielle du processeur
