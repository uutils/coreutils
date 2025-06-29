base32-about = encoder/décoder les données et les imprimer sur la sortie standard
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.

  Les données sont encodées comme décrit pour l'alphabet base32 dans RFC 4648.
  Lors du décodage, l'entrée peut contenir des retours à la ligne en plus
  des octets de l'alphabet base32 formel. Utilisez --ignore-garbage
  pour tenter de récupérer des autres octets non-alphabétiques dans
  le flux encodé.
base32-usage = base32 [OPTION]... [FICHIER]

# Messages d'erreur
base32-extra-operand = opérande supplémentaire {$operand}
base32-no-such-file = {$file} : Aucun fichier ou répertoire de ce type
base32-invalid-wrap-size = taille de retour à la ligne invalide : {$size}
base32-read-error = erreur de lecture : {$error}

# Messages d'aide
base32-help-decode = décoder les données
base32-help-ignore-garbage = lors du décodage, ignorer les caractères non-alphabétiques
base32-help-wrap = retour à la ligne des lignes encodées après COLS caractères (par défaut {$default}, 0 pour désactiver le retour à la ligne)
