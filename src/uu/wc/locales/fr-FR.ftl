wc-about = Afficher le nombre de nouvelles lignes, de mots et d'octets pour chaque FICHIER, et une ligne totale si plus d'un FICHIER est spécifié.
wc-usage = wc [OPTION]... [FICHIER]...

# Messages d'aide
wc-help-bytes = afficher le nombre d'octets
wc-help-chars = afficher le nombre de caractères
wc-help-files0-from = lire l'entrée depuis les fichiers spécifiés par
  des noms terminés par NUL dans le fichier F ;
  Si F est - alors lire les noms depuis l'entrée standard
wc-help-lines = afficher le nombre de nouvelles lignes
wc-help-max-line-length = afficher la longueur de la ligne la plus longue
wc-help-total = quand afficher une ligne avec les totaux ;
  WHEN peut être : auto, always, only, never
wc-help-words = afficher le nombre de mots

# Messages d'erreur
wc-error-files-disabled = opérande supplémentaire { $extra }
  les opérandes de fichier ne peuvent pas être combinées avec --files0-from
wc-error-stdin-repr-not-allowed = lors de la lecture des noms de fichiers depuis l'entrée standard, aucun nom de fichier '-' autorisé
wc-error-zero-length-filename = nom de fichier de longueur nulle invalide
wc-error-zero-length-filename-ctx = { $path }:{ $idx } : nom de fichier de longueur nulle invalide
wc-error-cannot-open-for-reading = impossible d'ouvrir { $path } en lecture
wc-error-read-error = { $path } : erreur de lecture
wc-error-failed-to-print-result = échec de l'affichage du résultat pour { $title }
wc-error-failed-to-print-total = échec de l'affichage du total

# Messages d'erreur du décodeur
decoder-error-invalid-byte-sequence = séquence d'octets invalide : { $bytes }
decoder-error-io = erreur du flux d'octets sous-jacent : { $error }

# Autres messages
wc-standard-input = entrée standard
wc-total = total

# Messages de débogage
wc-debug-hw-unavailable = debug : prise en charge matérielle indisponible sur ce CPU
wc-debug-hw-using = debug : utilisation de l'accélération matérielle (fonctions : { $features })
wc-debug-hw-disabled-env = debug : prise en charge matérielle désactivée par l'environnement
wc-debug-hw-disabled-glibc = debug : prise en charge matérielle désactivée par GLIBC_TUNABLES ({ $features })
wc-debug-hw-limited-glibc = debug : prise en charge matérielle limitée par GLIBC_TUNABLES (désactivé : { $disabled } ; activé : { $enabled })
