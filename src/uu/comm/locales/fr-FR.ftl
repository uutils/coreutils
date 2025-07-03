comm-about = Comparer deux fichiers triés ligne par ligne.

  Lorsque FICHIER1 ou FICHIER2 (pas les deux) est -, lire l'entrée standard.

  Sans options, produit une sortie à trois colonnes. La colonne un contient
  les lignes uniques à FICHIER1, la colonne deux contient les lignes uniques à FICHIER2,
  et la colonne trois contient les lignes communes aux deux fichiers.
comm-usage = comm [OPTION]... FICHIER1 FICHIER2

# Messages d'aide
comm-help-column-1 = supprimer la colonne 1 (lignes uniques à FICHIER1)
comm-help-column-2 = supprimer la colonne 2 (lignes uniques à FICHIER2)
comm-help-column-3 = supprimer la colonne 3 (lignes qui apparaissent dans les deux fichiers)
comm-help-delimiter = séparer les colonnes avec STR
comm-help-zero-terminated = le délimiteur de ligne est NUL, pas nouvelle ligne
comm-help-total = afficher un résumé
comm-help-check-order = vérifier que l'entrée est correctement triée, même si toutes les lignes d'entrée sont appariables
comm-help-no-check-order = ne pas vérifier que l'entrée est correctement triée

# Messages d'erreur
comm-error-file-not-sorted = comm : le fichier { $file_num } n'est pas dans l'ordre trié
comm-error-input-not-sorted = comm : l'entrée n'est pas dans l'ordre trié
comm-error-is-directory = Est un répertoire
comm-error-multiple-conflicting-delimiters = plusieurs délimiteurs de sortie en conflit spécifiés

# Autres messages
comm-total = total
