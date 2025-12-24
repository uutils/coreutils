df-about = afficher des informations sur le système de fichiers sur lequel chaque FICHIER réside,
  ou tous les systèmes de fichiers par défaut.
df-usage = df [OPTION]... [FICHIER]...
df-after-help = Les valeurs affichées sont en unités de la première TAILLE disponible de --block-size,
  et des variables d'environnement DF_BLOCK_SIZE, BLOCK_SIZE et BLOCKSIZE.
  Sinon, les unités par défaut sont 1024 octets (ou 512 si POSIXLY_CORRECT est défini).

  TAILLE est un entier et une unité optionnelle (exemple : 10M est 10*1024*1024).
  Les unités sont K, M, G, T, P, E, Z, Y (puissances de 1024) ou KB, MB,... (puissances
  de 1000). Les unités peuvent être décimales, hexadécimales, octales, binaires.

# Messages d'aide
df-help-print-help = afficher les informations d'aide.
df-help-all = inclure les systèmes de fichiers factices
df-help-block-size = mettre les tailles à l'échelle par TAILLE avant de les afficher ; par ex. '-BM' affiche les tailles en unités de 1 048 576 octets
df-help-total = produire un total général
df-help-human-readable = afficher les tailles dans un format lisible par l'homme (par ex., 1K 234M 2G)
df-help-si = pareillement, mais utiliser les puissances de 1000 pas 1024
df-help-inodes = lister les informations d'inode au lieu de l'utilisation des blocs
df-help-kilo = comme --block-size=1K
df-help-local = limiter l'affichage aux systèmes de fichiers locaux
df-help-no-sync = ne pas invoquer sync avant d'obtenir les informations d'utilisation (par défaut)
df-help-output = utiliser le format de sortie défini par LISTE_CHAMPS, ou afficher tous les champs si LISTE_CHAMPS est omise.
df-help-portability = utiliser le format de sortie POSIX
df-help-sync = invoquer sync avant d'obtenir les informations d'utilisation (non-windows seulement)
df-help-type = limiter l'affichage aux systèmes de fichiers de type TYPE
df-help-print-type = afficher le type de système de fichiers
df-help-exclude-type = limiter l'affichage aux systèmes de fichiers pas de type TYPE

# Messages d'erreur
df-error-block-size-too-large = argument --block-size '{ $size }' trop grand
df-error-invalid-block-size = argument --block-size invalide { $size }
df-error-invalid-suffix = suffixe invalide dans l'argument --block-size { $size }
df-error-field-used-more-than-once = option --output : champ { $field } utilisé plus d'une fois
df-error-filesystem-type-both-selected-and-excluded = type de système de fichiers { $type } à la fois sélectionné et exclu
df-error-no-such-file-or-directory = { $path } : aucun fichier ou répertoire de ce type
df-error-no-file-systems-processed = aucun système de fichiers traité
df-error-cannot-access-over-mounted = impossible d'accéder à { $path } : sur-monté par un autre périphérique
df-error-cannot-read-table-of-mounted-filesystems = impossible de lire la table des systèmes de fichiers montés
df-error-inodes-not-supported-windows = { $program } : ne supporte pas l'option -i

# En-têtes du tableau
df-header-filesystem = Sys. de fichiers
df-header-size = Taille
df-header-used = Utilisé
df-header-avail = Disp.
df-header-available = Disponible
df-header-use-percent = Util%
df-header-capacity = Capacité
df-header-mounted-on = Monté sur
df-header-inodes = Inodes
df-header-iused = IUtil
df-header-iavail = ILibre
df-header-iuse-percent = IUtil%
df-header-file = Fichier
df-header-type = Type

# Autres messages
df-total = total
df-blocks-suffix = -blocs
