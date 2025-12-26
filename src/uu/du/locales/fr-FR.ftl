du-about = Estimer l'utilisation de l'espace disque des fichiers
du-usage = du [OPTION]... [FICHIER]...
  du [OPTION]... --files0-from=F
du-after-help = Les valeurs affichées sont en unités de la première TAILLE disponible de --block-size,
  et des variables d'environnement DU_BLOCK_SIZE, BLOCK_SIZE et BLOCKSIZE.
  Sinon, les unités par défaut sont 1024 octets (ou 512 si POSIXLY_CORRECT est défini).

  TAILLE est un entier et une unité optionnelle (exemple : 10M est 10*1024*1024).
  Les unités sont K, M, G, T, P, E, Z, Y (puissances de 1024) ou KB, MB,... (puissances
  de 1000). Les unités peuvent être décimales, hexadécimales, octales, binaires.

  MOTIF permet des exclusions avancées. Par exemple, les syntaxes suivantes
  sont supportées :
  ? correspondra à un seul caractère
  { "*" } correspondra à zéro ou plusieurs caractères
  {"{"}a,b{"}"} correspondra à a ou b

# Messages d'aide
du-help-print-help = Afficher les informations d'aide.
du-help-all = afficher les comptes pour tous les fichiers, pas seulement les répertoires
du-help-apparent-size = afficher les tailles apparentes, plutôt que l'utilisation du disque bien que la taille apparente soit généralement plus petite, elle peut être plus grande en raison de trous dans les fichiers ('sparse'), la fragmentation interne, les blocs indirects, etc.
du-help-block-size = mettre à l'échelle les tailles par TAILLE avant de les afficher. Par ex., '-BM' affiche les tailles en unités de 1 048 576 octets. Voir le format TAILLE ci-dessous.
du-help-bytes = équivalent à '--apparent-size --block-size=1'
du-help-total = produire un total général
du-help-max-depth = afficher le total pour un répertoire (ou fichier, avec --all) seulement s'il est à N niveaux ou moins sous l'argument de ligne de commande ; --max-depth=0 est identique à --summarize
du-help-human-readable = afficher les tailles dans un format lisible par l'homme (p. ex., 1K 234M 2G)
du-help-inodes = lister les informations d'utilisation des inodes au lieu de l'utilisation des blocs comme --block-size=1K
du-help-block-size-1k = comme --block-size=1K
du-help-count-links = compter les tailles plusieurs fois si liées en dur
du-help-dereference = suivre tous les liens symboliques
du-help-dereference-args = suivre seulement les liens symboliques qui sont listés sur la ligne de commande
du-help-no-dereference = ne pas suivre les liens symboliques (c'est le défaut)
du-help-block-size-1m = comme --block-size=1M
du-help-null = terminer chaque ligne de sortie avec un octet 0 plutôt qu'une nouvelle ligne
du-help-separate-dirs = ne pas inclure la taille des sous-répertoires
du-help-summarize = afficher seulement un total pour chaque argument
du-help-si = comme -h, mais utiliser les puissances de 1000 et non 1024
du-help-one-file-system = ignorer les répertoires sur des systèmes de fichiers différents
du-help-threshold = exclure les entrées plus petites que TAILLE si positive, ou les entrées plus grandes que TAILLE si négative
du-help-verbose = mode verbeux (option non présente dans GNU/Coreutils)
du-help-exclude = exclure les fichiers qui correspondent au MOTIF
du-help-exclude-from = exclure les fichiers qui correspondent à n'importe quel motif dans FICHIER
du-help-files0-from = résumer l'utilisation du périphérique des noms de fichiers terminés par NUL spécifiés dans le fichier F ; si F est -, alors lire les noms depuis l'entrée standard
du-help-time = montrer l'heure de la dernière modification de n'importe quel fichier dans le répertoire, ou n'importe lequel de ses sous-répertoires. Si MOT est donné, montrer l'heure comme MOT au lieu de l'heure de modification : atime, access, use, ctime, status, birth ou creation
du-help-time-style = montrer les heures en utilisant le style STYLE : full-iso, long-iso, iso, +FORMAT FORMAT est interprété comme 'date'

# Messages d'erreur
du-error-invalid-max-depth = profondeur maximale invalide { $depth }
du-error-summarize-depth-conflict = la synthèse entre en conflit avec --max-depth={ $depth }
du-error-invalid-time-style = argument invalide { $style } pour 'style de temps'
  Les arguments valides sont :
    - 'full-iso'
    - 'long-iso'
    - 'iso'
    - +FORMAT (e.g., +%H:%M) pour un format de type 'date'
  Essayez '{ $help }' pour plus d'informations.
du-error-invalid-time-arg = les arguments 'birth' et 'creation' pour --time ne sont pas supportés sur cette plateforme.
du-error-invalid-glob = Syntaxe d'exclusion invalide : { $error }
du-error-cannot-read-directory = impossible de lire le répertoire { $path }
du-error-cannot-access = impossible d'accéder à { $path }
du-error-read-error-is-directory = { $file } : erreur de lecture : C'est un répertoire
du-error-cannot-open-for-reading = impossible d'ouvrir { $file } en lecture : Aucun fichier ou répertoire de ce type
du-error-invalid-zero-length-file-name = { $file }:{ $line } : nom de fichier de longueur zéro invalide
du-error-extra-operand-with-files0-from = opérande supplémentaire { $file }
  les opérandes de fichier ne peuvent pas être combinées avec --files0-from
du-error-invalid-block-size-argument = argument --{ $option } invalide { $value }
du-error-cannot-access-no-such-file = impossible d'accéder à { $path } : Aucun fichier ou répertoire de ce type
du-error-printing-thread-panicked = Le thread d'affichage a paniqué.
du-error-invalid-suffix = suffixe invalide dans l'argument --{ $option } { $value }
du-error-invalid-argument = argument --{ $option } invalide { $value }
du-error-argument-too-large = argument --{ $option } { $value } trop grand
du-error-hyphen-file-name-not-allowed = le nom de fichier '-' n'est pas autorisé lors de la lecture de l'entrée standard

# Messages verbeux/de statut
du-verbose-ignored = { $path } ignoré
du-verbose-adding-to-exclude-list = ajout de { $pattern } à la liste d'exclusion
du-total = total
du-warning-apparent-size-ineffective-with-inodes = les options --apparent-size et -b sont inefficaces avec --inodes
