split-about = Crée des fichiers de sortie contenant des sections consécutives ou alternées de l'entrée
split-usage = split [OPTION]... [INPUT [PREFIX]]
split-after-help =
    Écris des morceaux de taille fixe de INPUT vers PREFIXaa, PREFIXab, ... ; la taille par défaut est 1000 et le PREFIX par défaut est 'x'. Sans INPUT, ou quand INPUT = '-', lis l'entrée standard.

    L'argument SIZE est un entier avec une unité optionnelle (exemple : 10K = 10*1024).
    Les unités sont K,M,G,T,P,E,Z,Y,R,Q (puissances de 1024) ou KB,MB,... (puissances de 1000).
    Les préfixes binaires peuvent aussi être utilisés : KiB=K, MiB=M, et ainsi de suite.

    CHUNKS peut être :

    - N divise en N fichiers basé sur la taille de l'entrée
    - K/N envoie le K-ième de N vers stdout
    - l/N divise en N fichiers sans diviser les lignes/enregistrements
    - l/K/N envoie le K-ième de N vers stdout sans diviser les lignes/enregistrements
    - r/N comme 'l' mais utilise la distribution round-robin
    - r/K/N idem mais n'envoie que le K-ième de N vers stdout
# Messages d'erreur
split-error-suffix-not-parsable = longueur de suffixe invalide : { $value }
split-error-suffix-contains-separator = suffixe invalide { $value }, contient un séparateur de répertoire
split-error-suffix-too-small = la longueur du suffixe doit être au minimum { $length }
split-error-multi-character-separator = multi-caractère de séparation { $separator }
split-error-multiple-separator-characters = plusieurs caractères de séparation spécifiés
split-error-filter-with-kth-chunk = --filter ne traite pas les blocs redirigés vers stdout
split-error-invalid-io-block-size = taille de blocs IO invalide : { $size }
split-error-not-supported = --filter n'est actuellement pas supporté sur cette plateforme
split-error-invalid-number-of-chunks = nombre de blocs invalide : { $chunks }
split-error-invalid-chunk-number = numéro du bloc invalide : { $chunk }
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
split-error-file-descriptor-limit = limite de descripteurs de fichiers atteinte, mais aucun descripteur de fichier à fermer. { $count } flux d’écriture fermés auparavant.
split-error-shell-process-returned = Le processus shell a retourné { $code }
split-error-shell-process-terminated = Le processus shell a été terminé par un signal
# Messages d'aide pour les options de ligne de commande
split-help-bytes = écrit SIZE octets par fichier de sortie
split-help-line-bytes = écrit au maximum SIZE octets de lignes par fichier de sortie
split-help-lines = écrit NUMBER lignes/enregistrements par fichier de sortie
split-help-number = génère CHUNKS fichiers de sortie ; voir l'explication ci-dessous
split-help-additional-suffix = SUFFIX supplémentaire à ajouter aux noms de fichiers de sortie
split-help-filter = redirige la sortie vers la COMMAND shell ; le nom du fichier est $FILE (Actuellement non implémenté pour Windows)
split-help-elide-empty-files = ne génère pas de fichiers de sortie vides avec '-n'
split-help-numeric-suffixes-short = utilise des suffixes numériques commençant à 0, pas alphabétiques
split-help-numeric-suffixes = identique à -d, mais permet de définir la valeur de départ
split-help-hex-suffixes-short = utilise des suffixes hexadécimaux commençant à 0, pas alphabétiques
split-help-hex-suffixes = identique à -x, mais permet de définir la valeur de départ
split-help-suffix-length = génère des suffixes de longueur N (par défaut 2)
split-help-verbose = affiche un diagnostic juste avant l'ouverture de chaque fichier de sortie
split-help-separator = utiliser SEP au lieu du saut de ligne comme séparateur d'enregistrement ; '\\0' (zéro) spécifie le caractère NUL
