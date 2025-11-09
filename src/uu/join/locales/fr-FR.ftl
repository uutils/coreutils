join-about = Pour chaque paire de lignes d'entrée avec des champs de jointure identiques, écrire une ligne
  sur la sortie standard. Le champ de jointure par défaut est le premier, délimité par des espaces.

  Quand FILE1 ou FILE2 (mais pas les deux) est -, lire l'entrée standard.
join-usage = join [OPTION]... FICHIER1 FICHIER2

# Messages d'aide de join
join-help-a = afficher aussi les lignes non appariables du fichier NUMÉRO_FICHIER, où
  NUMÉRO_FICHIER est 1 ou 2, correspondant à FICHIER1 ou FICHIER2
join-help-v = comme -a NUMÉRO_FICHIER, mais supprimer les lignes de sortie jointes
join-help-e = remplacer les champs d'entrée manquants par VIDE
join-help-i = ignorer les différences de casse lors de la comparaison des champs
join-help-j = équivalent à '-1 CHAMP -2 CHAMP'
join-help-o = obéir au FORMAT lors de la construction de la ligne de sortie
join-help-t = utiliser CHAR comme séparateur de champ d'entrée et de sortie
join-help-1 = joindre sur ce CHAMP du fichier 1
join-help-2 = joindre sur ce CHAMP du fichier 2
join-help-check-order = vérifier que l'entrée est correctement triée, même si toutes les lignes d'entrée sont appariables
join-help-nocheck-order = ne pas vérifier que l'entrée est correctement triée
join-help-header = traiter la première ligne de chaque fichier comme des en-têtes de champs, les imprimer sans essayer de les apparier
join-help-z = le délimiteur de ligne est NUL, pas de nouvelle ligne

# Messages d'erreur de join
join-error-io = erreur d'E/S : { $error }
join-error-non-utf8-tab = tabulation multi-octets non-UTF-8
join-error-unprintable-separators = les séparateurs de champs non imprimables ne sont pris en charge que sur les plateformes de type unix
join-error-multi-character-tab = tabulation multi-caractères { $value }
join-error-both-files-stdin = les deux fichiers ne peuvent pas être l'entrée standard
join-error-invalid-field-specifier = spécificateur de champ invalide : { $spec }
join-error-invalid-file-number = numéro de fichier invalide dans la spécification de champ : { $spec }
join-error-invalid-file-number-simple = numéro de fichier invalide : { $value }
join-error-invalid-field-number = numéro de champ invalide : { $value }
join-error-incompatible-fields = champs de jointure incompatibles { $field1 }, { $field2 }
join-error-not-sorted = { $file }:{ $line_num } : n'est pas trié : { $content }
join-error-input-not-sorted = l'entrée n'est pas dans l'ordre trié
