sort-about = Affiche la concaténation triée de tous les FICHIER(s). Sans FICHIER, ou quand FICHIER est -, lit l'entrée standard.
sort-usage = sort [OPTION]... [FICHIER]...
sort-after-help = Le format de clé est CHAMP[.CAR][OPTIONS][,CHAMP[.CAR]][OPTIONS].

  Les champs sont séparés par défaut par le premier espace blanc après un caractère non-espace. Utilisez -t pour spécifier un séparateur personnalisé.
  Dans le cas par défaut, les espaces blancs sont ajoutés au début de chaque champ. Les séparateurs personnalisés ne sont cependant pas inclus dans les champs.

  CHAMP et CAR commencent tous deux à 1 (c'est-à-dire qu'ils sont indexés à partir de 1). S'il n'y a pas de fin spécifiée après une virgule, la fin sera la fin de la ligne.
  Si CAR est défini à 0, cela signifie la fin du champ. CAR par défaut à 1 pour la position de début et à 0 pour la position de fin.

  Les options valides sont : MbdfhnRrV. Elles remplacent les options globales pour cette clé.

# Messages d'erreur
sort-open-failed = échec d'ouverture : {$path} : {$error}
sort-parse-key-error = échec d'analyse de la clé {$key} : {$msg}
sort-cannot-read = impossible de lire : {$path} : {$error}
sort-open-tmp-file-failed = échec d'ouverture du fichier temporaire : {$error}
sort-compress-prog-execution-failed = impossible d'exécuter le programme de compression '{$prog}' : {$error}
sort-compress-prog-terminated-abnormally = {$prog} s'est terminé anormalement
sort-cannot-create-tmp-file = impossible de créer un fichier temporaire dans {$path} :
sort-file-operands-combined = opérande supplémentaire {$file}
    les opérandes de fichier ne peuvent pas être combinées avec --files0-from
    Essayez '{$help} --help' pour plus d'informations.
sort-multiple-output-files = plusieurs fichiers de sortie spécifiés
sort-minus-in-stdin = lors de la lecture des noms de fichiers depuis l'entrée standard, aucun nom de fichier '-' n'est autorisé
sort-no-input-from = aucune entrée depuis {$file}
sort-invalid-zero-length-filename = {$file}:{$line_num} : nom de fichier de longueur zéro invalide
sort-options-incompatible = les options '-{$opt1}{$opt2}' sont incompatibles
sort-invalid-key = clé invalide {$key}
sort-failed-parse-field-index = échec d'analyse de l'index de champ {$field} {$error}
sort-field-index-cannot-be-zero = l'index de champ ne peut pas être 0
sort-failed-parse-char-index = échec d'analyse de l'index de caractère {$char} : {$error}
sort-invalid-option = option invalide : '{$option}'
sort-invalid-char-index-zero-start = index de caractère 0 invalide pour la position de début d'un champ
sort-invalid-batch-size-arg = argument --batch-size invalide '{$arg}'
sort-minimum-batch-size-two = l'argument --batch-size minimum est '2'
sort-batch-size-too-large = argument --batch-size {$arg} trop grand
sort-maximum-batch-size-rlimit = argument --batch-size maximum avec la rlimit actuelle est {$rlimit}
sort-extra-operand-not-allowed-with-c = opérande supplémentaire {$operand} non autorisée avec -c
sort-separator-not-valid-unicode = le séparateur n'est pas un unicode valide : {$arg}
sort-separator-must-be-one-char = le séparateur doit faire exactement un caractère de long : {$separator}
sort-only-one-file-allowed-with-c = un seul fichier autorisé avec -c
sort-failed-fetch-rlimit = Échec de récupération de rlimit
sort-invalid-suffix-in-option-arg = suffixe invalide dans l'argument --{$option} {$arg}
sort-invalid-option-arg = argument --{$option} invalide {$arg}
sort-option-arg-too-large = argument --{$option} {$arg} trop grand
sort-error-disorder = {$file}:{$line_number}: désordre : {$line}
sort-error-buffer-size-too-big = La taille du tampon {$size} ne rentre pas dans l'espace d'adressage
sort-error-no-match-for-key = ^ aucune correspondance pour la clé
sort-error-write-failed = échec d'écriture : {$output}
sort-failed-to-delete-temporary-directory = échec de suppression du répertoire temporaire : {$error}
sort-failed-to-set-up-signal-handler = échec de configuration du gestionnaire de signal : {$error}

# Messages d'aide
sort-help-help = Affiche les informations d'aide.
sort-help-version = Affiche les informations de version.
sort-help-human-numeric = compare selon les tailles lisibles par l'humain, par ex. 1M > 100k
sort-help-month = compare selon l'abréviation du nom du mois
sort-help-numeric = compare selon la valeur numérique de la chaîne
sort-help-general-numeric = compare selon la valeur numérique générale de la chaîne
sort-help-version-sort = Trie par numéro de version SemVer, par ex. 1.12.2 > 1.1.2
sort-help-random = mélange dans un ordre aléatoire
sort-help-dictionary-order = considère seulement les espaces et les caractères alphanumériques
sort-help-merge = fusionne les fichiers déjà triés ; ne trie pas
sort-help-check = vérifie l'entrée triée ; ne trie pas
sort-help-check-silent = réussit si le fichier donné est déjà trié, et sort avec le statut 1 sinon.
sort-help-ignore-case = convertit les caractères minuscules en majuscules
sort-help-ignore-nonprinting = ignore les caractères non-imprimables
sort-help-ignore-leading-blanks = ignore les espaces de début lors de la recherche de clés de tri dans chaque ligne
sort-help-output = écrit la sortie vers NOMFICHIER au lieu de stdout
sort-help-reverse = inverse la sortie
sort-help-stable = stabilise le tri en désactivant la comparaison de dernier recours
sort-help-unique = affiche seulement le premier d'une série égale
sort-help-key = trie par une clé
sort-help-separator = séparateur personnalisé pour -k
sort-help-zero-terminated = le délimiteur de ligne est NUL, pas nouvelle ligne
sort-help-parallel = change le nombre de threads s'exécutant simultanément vers NUM_THREADS
sort-help-buf-size = définit la TAILLE maximale de chaque segment en nombre d'éléments triés
sort-help-tmp-dir = utilise RÉP pour les temporaires, pas $TMPDIR ou /tmp
sort-help-compress-prog = compresse les fichiers temporaires avec PROG, décompresse avec PROG -d ; PROG doit prendre l'entrée depuis stdin et sortir vers stdout
sort-help-batch-size = Fusionne au maximum N_MERGE entrées à la fois.
sort-help-files0-from = lit l'entrée depuis les fichiers spécifiés par FICHIER_NUL terminé par NUL
sort-help-debug = souligne les parties de la ligne qui sont réellement utilisées pour le tri
