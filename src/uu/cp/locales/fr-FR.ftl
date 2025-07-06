cp-about = Copier SOURCE vers DEST, ou plusieurs SOURCE(s) vers RÉPERTOIRE.
cp-usage = cp [OPTION]... [-T] SOURCE DEST
  cp [OPTION]... SOURCE... RÉPERTOIRE
  cp [OPTION]... -t RÉPERTOIRE SOURCE...
cp-after-help = Ne pas copier un non-répertoire qui a une destination existante avec le même horodatage de modification ou plus récent ;
  à la place, ignorer silencieusement le fichier sans échec. Si les horodatages sont préservés, la comparaison est faite avec
  l'horodatage source tronqué aux résolutions du système de fichiers de destination et des appels système utilisés pour
  mettre à jour les horodatages ; cela évite le travail en double si plusieurs commandes cp -pu sont exécutées avec la même source
  et destination. Cette option est ignorée si l'option -n ou --no-clobber est également spécifiée. De plus, si
  --preserve=links est également spécifié (comme avec cp -au par exemple), cela aura la priorité ; par conséquent,
  selon l'ordre dans lequel les fichiers sont traités depuis la source, les fichiers plus récents dans la destination peuvent être remplacés,
  pour refléter les liens durs dans la source. ce qui donne plus de contrôle sur les fichiers existants dans la destination qui sont
  remplacés, et sa valeur peut être l'une des suivantes :

  - all C'est l'opération par défaut lorsqu'une option --update n'est pas spécifiée, et entraîne le remplacement de tous les fichiers existants dans la destination.
  - none Cela est similaire à l'option --no-clobber, en ce sens qu'aucun fichier dans la destination n'est remplacé, mais ignorer un fichier n'induit pas d'échec.
  - older C'est l'opération par défaut lorsque --update est spécifié, et entraîne le remplacement des fichiers s'ils sont plus anciens que le fichier source correspondant.

# Messages d'aide
cp-help-target-directory = copier tous les arguments SOURCE dans le répertoire cible
cp-help-no-target-directory = Traiter DEST comme un fichier régulier et non comme un répertoire
cp-help-interactive = demander avant d'écraser les fichiers
cp-help-link = créer des liens durs au lieu de copier
cp-help-no-clobber = ne pas écraser un fichier qui existe déjà
cp-help-recursive = copier les répertoires récursivement
cp-help-strip-trailing-slashes = supprimer les barres obliques finales de chaque argument SOURCE
cp-help-debug = expliquer comment un fichier est copié. Implique -v
cp-help-verbose = indiquer explicitement ce qui est fait
cp-help-symbolic-link = créer des liens symboliques au lieu de copier
cp-help-force = si un fichier de destination existant ne peut pas être ouvert, le supprimer et réessayer (cette option est ignorée lorsque l'option -n est également utilisée). Actuellement non implémenté pour Windows.
cp-help-remove-destination = supprimer chaque fichier de destination existant avant de tenter de l'ouvrir (contraste avec --force). Sur Windows, ne fonctionne actuellement que pour les fichiers inscriptibles.
cp-help-reflink = contrôler les copies clone/CoW. Voir ci-dessous
cp-help-attributes-only = Ne pas copier les données du fichier, juste les attributs
cp-help-preserve = Préserver les attributs spécifiés (par défaut : mode, propriété (unix uniquement), horodatages), si possible attributs supplémentaires : contexte, liens, xattr, all
cp-help-preserve-default = identique à --preserve=mode,ownership(unix uniquement),timestamps
cp-help-no-preserve = ne pas préserver les attributs spécifiés
cp-help-parents = utiliser le nom complet du fichier source sous RÉPERTOIRE
cp-help-no-dereference = ne jamais suivre les liens symboliques dans SOURCE
cp-help-dereference = toujours suivre les liens symboliques dans SOURCE
cp-help-cli-symbolic-links = suivre les liens symboliques de la ligne de commande dans SOURCE
cp-help-archive = Identique à -dR --preserve=all
cp-help-no-dereference-preserve-links = identique à --no-dereference --preserve=links
cp-help-one-file-system = rester sur ce système de fichiers
cp-help-sparse = contrôler la création de fichiers épars. Voir ci-dessous
cp-help-selinux = définir le contexte de sécurité SELinux du fichier de destination au type par défaut
cp-help-context = comme -Z, ou si CTX est spécifié, définir le contexte de sécurité SELinux ou SMACK à CTX
cp-help-progress = Afficher une barre de progression. Note : cette fonctionnalité n'est pas supportée par GNU coreutils.
cp-help-copy-contents = Non implémenté : copier le contenu des fichiers spéciaux lors de la récursion

# Messages d'erreur
cp-error-missing-file-operand = opérande fichier manquant
cp-error-missing-destination-operand = opérande fichier de destination manquant après { $source }
cp-error-extra-operand = opérande supplémentaire { $operand }
cp-error-same-file = { $source } et { $dest } sont le même fichier
cp-error-backing-up-destroy-source = sauvegarder { $dest } pourrait détruire la source ; { $source } non copié
cp-error-cannot-open-for-reading = impossible d'ouvrir { $source } en lecture
cp-error-not-writing-dangling-symlink = ne pas écrire à travers le lien symbolique pendant { $dest }
cp-error-failed-to-clone = échec du clonage de { $source } depuis { $dest } : { $error }
cp-error-cannot-change-attribute = impossible de changer l'attribut { $dest } : Le fichier source n'est pas un fichier régulier
cp-error-cannot-stat = impossible de faire stat sur { $source } : Aucun fichier ou répertoire de ce type
cp-error-cannot-create-symlink = impossible de créer le lien symbolique { $dest } vers { $source }
cp-error-cannot-create-hard-link = impossible de créer le lien dur { $dest } vers { $source }
cp-error-omitting-directory = -r non spécifié ; répertoire { $dir } omis
cp-error-cannot-copy-directory-into-itself = impossible de copier un répertoire, { $source }, dans lui-même, { $dest }
cp-error-will-not-copy-through-symlink = ne copiera pas { $source } à travers le lien symbolique tout juste créé { $dest }
cp-error-will-not-overwrite-just-created = n'écrasera pas le fichier tout juste créé { $dest } avec { $source }
cp-error-target-not-directory = cible : { $target } n'est pas un répertoire
cp-error-cannot-overwrite-directory-with-non-directory = impossible d'écraser le répertoire { $dir } avec un non-répertoire
cp-error-cannot-overwrite-non-directory-with-directory = impossible d'écraser un non-répertoire avec un répertoire
cp-error-with-parents-dest-must-be-dir = avec --parents, la destination doit être un répertoire
cp-error-not-replacing = ne remplace pas { $file }
cp-error-failed-get-current-dir = échec de l'obtention du répertoire actuel { $error }
cp-error-failed-set-permissions = impossible de définir les permissions { $path }
cp-error-backup-mutually-exclusive = les options --backup et --no-clobber sont mutuellement exclusives
cp-error-invalid-argument = argument invalide { $arg } pour '{ $option }'
cp-error-option-not-implemented = Option '{ $option }' pas encore implémentée.
cp-error-not-all-files-copied = Tous les fichiers n'ont pas été copiés
cp-error-reflink-always-sparse-auto = `--reflink=always` ne peut être utilisé qu'avec --sparse=auto
cp-error-file-exists = { $path } : Le fichier existe
cp-error-invalid-backup-argument = --backup est mutuellement exclusif avec -n ou --update=none-fail
cp-error-reflink-not-supported = --reflink n'est supporté que sur linux et macOS
cp-error-sparse-not-supported = --sparse n'est supporté que sur linux
cp-error-not-a-directory = { $path } n'est pas un répertoire
cp-error-selinux-not-enabled = SELinux n'était pas activé lors de la compilation !
cp-error-selinux-set-context = échec de la définition du contexte de sécurité de { $path } : { $error }
cp-error-selinux-get-context = échec de l'obtention du contexte de sécurité de { $path }
cp-error-selinux-error = Erreur SELinux : { $error }
cp-error-cannot-create-fifo = impossible de créer le fifo { $path } : Le fichier existe
cp-error-invalid-attribute = attribut invalide { $value }
cp-error-failed-to-create-whole-tree = échec de la création de l'arborescence complète
cp-error-failed-to-create-directory = Échec de la création du répertoire : { $error }
cp-error-backup-format = cp : { $error }
  Tentez '{ $exec } --help' pour plus d'informations.

# Debug enum strings
cp-debug-enum-no = non
cp-debug-enum-yes = oui
cp-debug-enum-avoided = évité
cp-debug-enum-unsupported = non supporté
cp-debug-enum-unknown = inconnu
cp-debug-enum-zeros = zéros
cp-debug-enum-seek-hole = SEEK_HOLE
cp-debug-enum-seek-hole-zeros = SEEK_HOLE + zéros

# Messages d'avertissement
cp-warning-source-specified-more-than-once = { $file_type } source { $source } spécifié plus d'une fois

# Messages verbeux et de débogage
cp-verbose-copied = { $source } -> { $dest }
cp-debug-skipped = { $path } ignoré
cp-verbose-created-directory = { $source } -> { $dest }
cp-debug-copy-offload = copy offload : { $offload }, reflink : { $reflink }, sparse detection : { $sparse }

# Invites
cp-prompt-overwrite = écraser { $path } ?
cp-prompt-overwrite-with-mode = remplacer { $path }, en écrasant le mode
