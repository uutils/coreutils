csplit-about = Diviser un fichier en sections déterminées par des lignes de contexte
csplit-usage = csplit [OPTION]... FICHIER MOTIF...
csplit-after-help = Sortir les morceaux de FICHIER séparés par MOTIF(S) dans les fichiers 'xx00', 'xx01', ..., et sortir le nombre d'octets de chaque morceau sur la sortie standard.

# Messages d'aide
csplit-help-suffix-format = utiliser le FORMAT sprintf au lieu de %02d
csplit-help-prefix = utiliser PRÉFIXE au lieu de 'xx'
csplit-help-keep-files = ne pas supprimer les fichiers de sortie en cas d'erreurs
csplit-help-suppress-matched = supprimer les lignes correspondant au MOTIF
csplit-help-digits = utiliser le nombre spécifié de chiffres au lieu de 2
csplit-help-quiet = ne pas afficher le nombre d'octets des fichiers de sortie
csplit-help-elide-empty-files = supprimer les fichiers de sortie vides

# Messages d'erreur
csplit-error-line-out-of-range = { $pattern } : numéro de ligne hors limites
csplit-error-line-out-of-range-on-repetition = { $pattern } : numéro de ligne hors limites à la répétition { $repetition }
csplit-error-match-not-found = { $pattern } : correspondance non trouvée
csplit-error-match-not-found-on-repetition = { $pattern } : correspondance non trouvée à la répétition { $repetition }
csplit-error-line-number-is-zero = 0 : le numéro de ligne doit être supérieur à zéro
csplit-error-line-number-smaller-than-previous = le numéro de ligne '{ $current }' est plus petit que le numéro de ligne précédent, { $previous }
csplit-error-invalid-pattern = { $pattern } : motif invalide
csplit-error-invalid-number = nombre invalide : { $number }
csplit-error-suffix-format-incorrect = spécification de conversion incorrecte dans le suffixe
csplit-error-suffix-format-too-many-percents = trop de spécifications de conversion % dans le suffixe
csplit-error-not-regular-file = { $file } n'est pas un fichier régulier
csplit-warning-line-number-same-as-previous = le numéro de ligne '{ $line_number }' est identique au numéro de ligne précédent
csplit-stream-not-utf8 = le flux ne contenait pas d'UTF-8 valide
csplit-read-error = erreur de lecture
csplit-write-split-not-created = tentative d'écriture dans une division qui n'a pas été créée
