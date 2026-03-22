numfmt-about = Convertir les nombres vers/depuis des chaînes lisibles par l'homme
numfmt-usage = numfmt [OPTION]... [NOMBRE]...
numfmt-after-help = Options d'UNITÉ :

  - none : aucune mise à l'échelle automatique n'est effectuée ; les suffixes déclencheront une erreur
  - auto : accepter un suffixe optionnel d'une/deux lettres :

      1K = 1000, 1Ki = 1024, 1M = 1000000, 1Mi = 1048576,

  - si : accepter un suffixe optionnel d'une lettre :

      1K = 1000, 1M = 1000000, ...

  - iec : accepter un suffixe optionnel d'une lettre :

      1K = 1024, 1M = 1048576, ...

  - iec-i : accepter un suffixe optionnel de deux lettres :

      1Ki = 1024, 1Mi = 1048576, ...

  - FIELDS supporte les plages de champs de style cut(1) :

      N N-ième champ, compté à partir de 1
      N- du N-ième champ jusqu'à la fin de la ligne
      N-M du N-ième au M-ième champ (inclus)
      -M du premier au M-ième champ (inclus)
      - tous les champs

  Plusieurs champs/plages peuvent être séparés par des virgules

  FORMAT doit être adapté pour imprimer un argument à virgule flottante %f.
  Une guillemet optionnelle (%'f) activera --grouping (si supporté par la locale actuelle).
  Une valeur de largeur optionnelle (%10f) remplira la sortie. Un zéro optionnel (%010f)
  remplira le nombre de zéros. Des valeurs négatives optionnelles (%-10f) aligneront à gauche.
  Une précision optionnelle (%.1f) remplacera la précision déterminée par l'entrée.

# Messages d'aide
numfmt-help-delimiter = utiliser X au lieu d'espaces pour le délimiteur de champ
numfmt-help-field = remplacer les nombres dans ces champs d'entrée ; voir FIELDS ci-dessous
numfmt-help-format = utiliser le FORMAT à virgule flottante de style printf ; voir FORMAT ci-dessous pour les détails
numfmt-help-from = mettre automatiquement à l'échelle les nombres d'entrée vers les UNITÉs ; voir UNIT ci-dessous
numfmt-help-from-unit = spécifier la taille de l'unité d'entrée
numfmt-help-to = mettre automatiquement à l'échelle les nombres de sortie vers les UNITÉs ; voir UNIT ci-dessous
numfmt-help-to-unit = la taille de l'unité de sortie
numfmt-help-padding = remplir la sortie à N caractères ; N positif alignera à droite ; N négatif alignera à gauche ; le remplissage est ignoré si la sortie est plus large que N ; la valeur par défaut est de remplir automatiquement si un espace est trouvé
numfmt-help-header = imprimer (sans convertir) les N premières lignes d'en-tête ; N vaut 1 par défaut si non spécifié
numfmt-help-round = utiliser METHOD pour l'arrondi lors de la mise à l'échelle
numfmt-help-suffix = imprimer SUFFIX après chaque nombre formaté, et accepter les entrées se terminant optionnellement par SUFFIX
numfmt-help-invalid = définir le mode d'échec pour les entrées invalides
numfmt-help-zero-terminated = le délimiteur de ligne est NUL, pas retour à la ligne

# Messages d'erreur
numfmt-error-unsupported-unit = Une unité non supportée est spécifiée
numfmt-error-invalid-unit-size = taille d'unité invalide : { $size }
numfmt-error-invalid-padding = valeur de remplissage invalide { $value }
numfmt-error-invalid-header = valeur d'en-tête invalide { $value }
numfmt-error-grouping-cannot-be-combined-with-to = le groupement ne peut pas être combiné avec --to
numfmt-error-delimiter-must-be-single-character = le délimiteur doit être un seul caractère
numfmt-error-invalid-number-empty = nombre invalide : ''
numfmt-error-invalid-suffix = suffixe invalide dans l'entrée : { $input }
numfmt-error-invalid-number = nombre invalide : { $input }
numfmt-error-missing-i-suffix = suffixe 'i' manquant dans l'entrée : '{ $number }{ $suffix }' (par ex. Ki/Mi/Gi)
numfmt-error-rejecting-suffix = rejet du suffixe dans l'entrée : '{ $number }{ $suffix }' (considérez utiliser --from)
numfmt-error-suffix-unsupported-for-unit = Ce suffixe n'est pas supporté pour l'unité spécifiée
numfmt-error-unit-auto-not-supported-with-to = L'unité 'auto' n'est pas supportée avec les options --to
numfmt-error-number-too-big = Le nombre est trop grand et non supporté
numfmt-error-format-no-percent = le format '{ $format }' n'a pas de directive %
numfmt-error-format-ends-in-percent = le format '{ $format }' se termine par %
numfmt-error-invalid-format-directive = format invalide '{ $format }', la directive doit être %[0]['][-][N][.][N]f
numfmt-error-invalid-format-width-overflow = format invalide '{ $format }' (débordement de largeur)
numfmt-error-invalid-precision = précision invalide dans le format '{ $format }'
numfmt-error-format-too-many-percent = le format '{ $format }' a trop de directives %
numfmt-error-unknown-invalid-mode = Mode invalide inconnu : { $mode }
