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
tr-warning-invalid-utf8 = séquence UTF-8 non valide

# Messages d'erreur d'analyse de séquence
tr-error-missing-char-class-name = nom de classe de caractères manquant '[::]'
tr-error-invalid-char-class = classe de caractères non valide { $class }
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

# Étiquettes de diagnostic : ce que le caret désigne dans un ensemble
tr-diag-label-missing-char-class-name = aucun nom de classe entre les crochets
tr-diag-label-invalid-char-class = n'est pas une classe de caractères
tr-diag-label-missing-equivalence-char = aucun caractère entre les crochets
tr-diag-label-multiple-char-in-equivalence = une classe d'équivalence contient un seul caractère
tr-diag-label-invalid-repeat-count = n'est pas un nombre de répétitions
tr-diag-label-backwards-range = cet intervalle est à l'envers
tr-diag-label-char-repeat-in-set1 = une répétition n'a de sens que dans SET2
tr-diag-label-multiple-char-repeat-in-set2 = un ensemble ne peut contenir qu'une répétition ouverte
tr-diag-label-class-except-lower-upper-in-set2 = seules [:lower:] et [:upper:] peuvent être une cible de traduction
tr-diag-label-class-in-set2-not-matched = aucune classe à la position correspondante dans SET1
tr-diag-label-set1-longer-set2-ends-in-class = cet ensemble est plus long que SET2
tr-diag-label-complement-more-than-one-unique = un seul caractère peut être la cible du complément
tr-diag-help-char-class = les classes sont alnum, alpha, blank, cntrl, digit, graph, lower, print, punct, space, upper et xdigit
tr-diag-help-equivalence = [=c=] désigne tout caractère équivalent à c
tr-diag-help-repeat = [c*N] répète c N fois, [c*] complète SET2 à la longueur de SET1
tr-diag-help-backwards-range = un intervalle va du caractère le plus petit au plus grand, comme a-z
