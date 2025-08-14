tr-about = Traduire ou supprimer des caractères
tr-usage = tr [OPTION]... ENSEMBLE1 [ENSEMBLE2]
tr-after-help = Traduire, compresser et/ou supprimer des caractères de l'entrée standard, en écrivant vers la sortie standard.

# Messages d'aide
tr-help-complement = utiliser le complément d'ENSEMBLE1
tr-help-delete = supprimer les caractères dans ENSEMBLE1, ne pas traduire
tr-help-squeeze = remplacer chaque séquence d'un caractère répété qui est listé dans le dernier ENSEMBLE spécifié, avec une seule occurrence de ce caractère
tr-help-truncate-set1 = d'abord tronquer ENSEMBLE1 à la longueur d'ENSEMBLE2

# Messages d'erreur
tr-error-missing-operand = opérande manquant
tr-error-missing-operand-translating = opérande manquant après { $set }
  Deux chaînes doivent être données lors de la traduction.
tr-error-missing-operand-deleting-squeezing = opérande manquant après { $set }
  Deux chaînes doivent être données lors de la suppression et compression.
tr-error-extra-operand-deleting-without-squeezing = opérande supplémentaire { $operand }
  Une seule chaîne peut être donnée lors de la suppression sans compression des répétitions.
tr-error-extra-operand-simple = opérande supplémentaire { $operand }
tr-error-read-directory = erreur de lecture : Est un répertoire
tr-error-write-error = erreur d'écriture

# Messages d'avertissement
tr-warning-unescaped-backslash = avertissement : une barre oblique inverse non échappée à la fin de la chaîne n'est pas portable
tr-warning-ambiguous-octal-escape = l'échappement octal ambigu \{ $origin_octal } est en cours
  d'interprétation comme la séquence de 2 octets \0{ $actual_octal_tail }, { $outstand_char }

# Messages d'erreur d'analyse de séquence
tr-error-missing-char-class-name = nom de classe de caractères manquant '[::]'
tr-error-missing-equivalence-class-char = caractère de classe d'équivalence manquant '[==]'
tr-error-multiple-char-repeat-in-set2 = seule une construction de répétition [c*] peut apparaître dans string2
tr-error-char-repeat-in-set1 = la construction de répétition [c*] ne peut pas apparaître dans string1
tr-error-invalid-repeat-count = nombre de répétitions invalide { $count } dans la construction [c*n]
tr-error-empty-set2-when-not-truncating = quand on ne tronque pas set1, string2 doit être non-vide
tr-error-class-except-lower-upper-in-set2 = lors de la traduction, les seules classes de caractères qui peuvent apparaître dans set2 sont 'upper' et 'lower'
tr-error-class-in-set2-not-matched = lors de la traduction, chaque 'upper'/'lower' dans set2 doit être associé à un 'upper'/'lower' à la même position dans set1
tr-error-set1-longer-set2-ends-in-class = lors de la traduction avec string1 plus long que string2,
  cette dernière chaîne ne doit pas se terminer par une classe de caractères
tr-error-complement-more-than-one-unique = lors de la traduction avec des classes de caractères complémentées,
  string2 doit mapper tous les caractères du domaine vers un seul
tr-error-backwards-range = les points de fin de plage de '{ $start }-{ $end }' sont dans l'ordre inverse de la séquence de collation
tr-error-multiple-char-in-equivalence = { $chars } : l'opérande de classe d'équivalence doit être un seul caractère
