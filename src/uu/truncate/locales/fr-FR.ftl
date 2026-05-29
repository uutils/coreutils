truncate-about = Réduire ou étendre la taille de chaque fichier à la taille spécifiée.
truncate-usage = truncate [OPTION]... [FICHIER]...
truncate-after-help = TAILLE est un entier avec un préfixe optionnel et une unité optionnelle.
  Les unités disponibles (K, M, G, T, P, E, Z, et Y) utilisent le format suivant :
      'KB' => 1000 (kilooctets)
      'K' => 1024 (kibioctets)
      'MB' => 1000*1000 (mégaoctets)
      'M' => 1024*1024 (mébioctets)
      'GB' => 1000*1000*1000 (gigaoctets)
      'G' => 1024*1024*1024 (gibioctets)
  TAILLE peut aussi être préfixée par l'un des éléments suivants pour ajuster la taille de chaque
  fichier basé sur sa taille actuelle :
      '+' => étendre de
      '-' => réduire de
      '<' => au maximum
      '>' => au minimum
      '/' => arrondir vers le bas au multiple de
      '%' => arrondir vers le haut au multiple de

# Messages d'aide
truncate-help-io-blocks = traiter TAILLE comme le nombre de blocs I/O du fichier plutôt que des octets (NON IMPLÉMENTÉ)
truncate-help-no-create = ne pas créer les fichiers qui n'existent pas
truncate-help-reference = baser la taille de chaque fichier sur la taille de RFICHIER
truncate-help-size = définir ou ajuster la taille de chaque fichier selon TAILLE, qui est en octets sauf si --io-blocks est spécifié

# Messages d'erreur
truncate-error-missing-file-operand = opérande de fichier manquant
truncate-error-cannot-open-no-device = impossible d'ouvrir { $filename } en écriture : Aucun périphérique ou adresse de ce type
truncate-error-cannot-open-for-writing = impossible d'ouvrir { $filename } en écriture
truncate-error-invalid-number = Nombre invalide : { $error }
truncate-error-must-specify-relative-size = vous devez spécifier une '--size' relative avec '--reference'
truncate-error-division-by-zero = division par zéro
truncate-error-cannot-stat-no-such-file = impossible d'obtenir les informations de { $filename } : Aucun fichier ou répertoire de ce type
