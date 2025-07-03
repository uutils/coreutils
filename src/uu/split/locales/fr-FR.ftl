split-about = Créer des fichiers de sortie contenant des sections consécutives ou entrelacées de l'entrée
split-usage = split [OPTION]... [ENTRÉE [PRÉFIXE]]
split-after-help = Sortir des morceaux de taille fixe d'ENTRÉE vers PREFIXEaa, PREFIXEab, ...; la taille par défaut est 1000, et le PRÉFIXE par défaut est 'x'. Sans ENTRÉE, ou quand ENTRÉE est -, lire l'entrée standard.

  L'argument TAILLE est un entier et une unité optionnelle (exemple: 10K est 10*1024).
  Les unités sont K,M,G,T,P,E,Z,Y,R,Q (puissances de 1024) ou KB,MB,... (puissances de 1000).
  Les préfixes binaires peuvent être utilisés aussi: KiB=K, MiB=M, et ainsi de suite.

  CHUNKS peut être:

  - N diviser en N fichiers basé sur la taille de l'entrée
  - K/N sortir le Kème de N vers stdout
  - l/N diviser en N fichiers sans diviser les lignes/enregistrements
  - l/K/N sortir le Kème de N vers stdout sans diviser les lignes/enregistrements
  - r/N comme 'l' mais utiliser la distribution round robin
  - r/K/N pareillement mais ne sortir que le Kème de N vers stdout

# Messages d'erreur
split-error-suffix-not-parsable = longueur de suffixe invalide : { $value }
split-error-suffix-contains-separator = suffixe invalide { $value }, contient un séparateur de répertoire
split-error-suffix-too-small = la longueur du suffixe doit être au moins { $length }
split-error-multi-character-separator = séparateur multi-caractères { $separator }
split-error-multiple-separator-characters = plusieurs caractères de séparateur spécifiés
split-error-filter-with-kth-chunk = --filter ne traite pas un chunk extrait vers stdout
split-error-invalid-io-block-size = taille de bloc IO invalide : { $size }
split-error-not-supported = --filter n'est actuellement pas supporté sur cette plateforme
split-error-invalid-number-of-chunks = nombre de chunks invalide : { $chunks }
split-error-invalid-chunk-number = numéro de chunk invalide : { $chunk }
split-error-invalid-number-of-lines = nombre de lignes invalide : { $error }
split-error-invalid-number-of-bytes = nombre d'octets invalide : { $error }
split-error-cannot-split-more-than-one-way = impossible de diviser de plus d'une façon
split-error-overflow = Débordement
split-error-output-file-suffixes-exhausted = suffixes de fichiers de sortie épuisés
split-error-numerical-suffix-start-too-large = la valeur de départ du suffixe numérique est trop grande pour la longueur du suffixe
split-error-cannot-open-for-reading = impossible d'ouvrir { $file } en lecture
split-error-would-overwrite-input = { $file } écraserait l'entrée ; abandon
split-error-cannot-determine-input-size = { $input } : impossible de déterminer la taille de l'entrée
split-error-cannot-determine-file-size = { $input } : impossible de déterminer la taille du fichier
split-error-cannot-read-from-input = { $input } : impossible de lire depuis l'entrée : { $error }
split-error-input-output-error = erreur d'entrée/sortie
split-error-unable-to-open-file = impossible d'ouvrir { $file } ; abandon
split-error-unable-to-reopen-file = impossible de rouvrir { $file } ; abandon
split-error-file-descriptor-limit = limite de descripteurs de fichiers atteinte, mais aucun descripteur de fichier à fermer. { $count } écrivains fermés auparavant.
split-error-shell-process-returned = Le processus shell a retourné { $code }
split-error-shell-process-terminated = Le processus shell a été terminé par un signal

# Messages d'aide pour les options de ligne de commande
split-help-bytes = mettre TAILLE octets par fichier de sortie
split-help-line-bytes = mettre au maximum TAILLE octets de lignes par fichier de sortie
split-help-lines = mettre NOMBRE lignes/enregistrements par fichier de sortie
split-help-number = générer CHUNKS fichiers de sortie ; voir l'explication ci-dessous
split-help-additional-suffix = SUFFIXE supplémentaire à ajouter aux noms de fichiers de sortie
split-help-filter = écrire vers la COMMANDE shell ; le nom de fichier est $FILE (Actuellement non implémenté pour Windows)
split-help-elide-empty-files = ne pas générer de fichiers de sortie vides avec '-n'
split-help-numeric-suffixes-short = utiliser des suffixes numériques commençant à 0, pas alphabétiques
split-help-numeric-suffixes = identique à -d, mais permet de définir la valeur de départ
split-help-hex-suffixes-short = utiliser des suffixes hexadécimaux commençant à 0, pas alphabétiques
split-help-hex-suffixes = identique à -x, mais permet de définir la valeur de départ
split-help-suffix-length = générer des suffixes de longueur N (par défaut 2)
split-help-verbose = afficher un diagnostic juste avant l'ouverture de chaque fichier de sortie
split-help-separator = utiliser SEP au lieu de nouvelle ligne comme séparateur d'enregistrement ; '\\0' (zéro) spécifie le caractère NUL
