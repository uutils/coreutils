unexpand-about = Convertir les espaces dans chaque FICHIER en tabulations, en écrivant vers la sortie standard.
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.
unexpand-usage = unexpand [OPTION]... [FICHIER]...

# Messages d'aide
unexpand-help-all = convertir tous les espaces, au lieu de seulement les espaces initiaux
unexpand-help-first-only = convertir seulement les séquences d'espaces en début de ligne (remplace -a)
unexpand-help-tabs = utiliser une LISTE séparée par des virgules de positions de tabulations ou avoir des tabulations de N caractères au lieu de 8 (active -a)
unexpand-help-no-utf8 = interpréter le fichier d'entrée comme ASCII 8-bit plutôt que UTF-8

# Messages d'erreur
unexpand-error-invalid-character = la taille de tabulation contient des caractères invalides : { $char }
unexpand-error-tab-size-cannot-be-zero = la taille de tabulation ne peut pas être 0
unexpand-error-tab-size-too-large = la valeur d'arrêt de tabulation est trop grande
unexpand-error-tab-sizes-must-be-ascending = les tailles de tabulation doivent être croissantes
unexpand-error-is-directory = { $path } : Est un répertoire
