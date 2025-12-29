ptx-about = Produire un index permuté du contenu des fichiers
  Sortir un index permuté, incluant le contexte, des mots dans les fichiers d'entrée.
  Les arguments obligatoires pour les options longues le sont aussi pour les options courtes.
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard. Par défaut c'est '-F /'.
ptx-usage = ptx [OPTION]... [ENTRÉE]...
  ptx -G [OPTION]... [ENTRÉE [SORTIE]]

# Messages d'aide
ptx-help-auto-reference = sortir les références générées automatiquement
ptx-help-traditional = se comporter plus comme le 'ptx' de System V
ptx-help-flag-truncation = utiliser CHAÎNE pour marquer les troncatures de ligne
ptx-help-macro-name = nom de macro à utiliser au lieu de 'xx'
ptx-help-roff = générer la sortie comme directives roff
ptx-help-tex = générer la sortie comme directives TeX
ptx-help-right-side-refs = mettre les références à droite, non comptées dans -w
ptx-help-sentence-regexp = pour la fin de lignes ou la fin de phrases
ptx-help-word-regexp = utiliser REGEXP pour correspondre à chaque mot-clé
ptx-help-break-file = caractères de coupure de mots dans ce FICHIER
ptx-help-ignore-case = replier les minuscules en majuscules pour le tri
ptx-help-gap-size = taille de l'écart en colonnes entre les champs de sortie
ptx-help-ignore-file = lire la liste de mots à ignorer depuis FICHIER
ptx-help-only-file = lire seulement la liste de mots depuis ce FICHIER
ptx-help-references = le premier champ de chaque ligne est une référence
ptx-help-width = largeur de sortie en colonnes, référence exclue

# Messages d'erreur
ptx-error-dumb-format = Il n'y a pas de format simple avec les extensions GNU désactivées
ptx-error-not-implemented = { $feature } pas encore implémenté
ptx-error-write-failed = échec de l'écriture
ptx-error-extra-operand = opérande supplémentaire { $operand }
ptx-error-empty-regexp = Une expression régulière ne peut pas correspondre à une chaîne de longueur zéro
ptx-error-invalid-regexp = Expression régulière invalide : { $error }
