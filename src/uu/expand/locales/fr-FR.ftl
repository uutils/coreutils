expand-about = Convertir les tabulations de chaque FICHIER en espaces, en écrivant vers la sortie standard.
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.
expand-usage = expand [OPTION]... [FICHIER]...

# Messages d'aide
expand-help-initial = ne pas convertir les tabulations après les caractères non-blancs
expand-help-tabs = avoir des tabulations espacées de N caractères, pas 8 ou utiliser une liste séparée par des virgules de positions de tabulation explicites
expand-help-no-utf8 = interpréter le fichier d'entrée comme ASCII 8 bits plutôt que UTF-8

# Messages d'erreur
expand-error-invalid-character = la taille de tabulation contient des caractères invalides : { $char }
expand-error-specifier-not-at-start = le spécificateur { $specifier } n'est pas au début du nombre : { $number }
expand-error-specifier-only-allowed-with-last = le spécificateur { $specifier } n'est autorisé qu'avec la dernière valeur
expand-error-tab-size-cannot-be-zero = la taille de tabulation ne peut pas être 0
expand-error-tab-size-too-large = l'arrêt de tabulation est trop grand { $size }
expand-error-tab-sizes-must-be-ascending = les tailles de tabulation doivent être croissantes
expand-error-is-directory = { $file } : Est un répertoire
expand-error-failed-to-write-output = échec de l'écriture de la sortie
