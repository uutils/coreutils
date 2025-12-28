mv-about = Déplacer SOURCE vers DEST, ou plusieurs SOURCE(s) vers RÉPERTOIRE.
mv-usage = mv [OPTION]... [-T] SOURCE DEST
  mv [OPTION]... SOURCE... RÉPERTOIRE
  mv [OPTION]... -t RÉPERTOIRE SOURCE...
mv-after-help = Lors de la spécification de plus d'une option parmi -i, -f, -n, seule la dernière prend effet.

  Ne pas déplacer un non-répertoire qui a une destination existante avec un horodatage de modification identique ou plus récent ;
  au lieu de cela, ignorer silencieusement le fichier sans échouer. Si le déplacement traverse les limites du système de fichiers, la comparaison est
  avec l'horodatage source tronqué aux résolutions du système de fichiers de destination et des appels système utilisés
  pour mettre à jour les horodatages ; cela évite le travail en double si plusieurs commandes mv -u sont exécutées avec la même source
  et destination. Cette option est ignorée si l'option -n ou --no-clobber est également spécifiée, qui donne plus de contrôle
  sur quels fichiers existants dans la destination sont remplacés, et sa valeur peut être une des suivantes :

  - all C'est l'opération par défaut quand une option --update n'est pas spécifiée, et résulte en tous les fichiers existants dans la destination étant remplacés.
  - none C'est similaire à l'option --no-clobber, en ce que aucun fichier dans la destination n'est remplacé, mais aussi ignorer un fichier n'induit pas un échec.
  - older C'est l'opération par défaut quand --update est spécifié, et résulte en des fichiers étant remplacés s'ils sont plus anciens que le fichier source correspondant.

# Messages d'erreur
mv-error-insufficient-arguments = L'argument '<{$arg_files}>...' nécessite au moins 2 valeurs, mais seulement 1 a été fournie
mv-error-no-such-file = impossible de lire {$path} : Aucun fichier ou répertoire de ce nom
mv-error-cannot-stat-not-directory = impossible de lire {$path} : N'est pas un répertoire
mv-error-same-file = {$source} et {$target} sont le même fichier
mv-error-self-target-subdirectory = impossible de déplacer {$source} vers un sous-répertoire de lui-même, {$target}
mv-error-directory-to-non-directory = impossible d'écraser le répertoire {$path} avec un non-répertoire
mv-error-non-directory-to-directory = impossible d'écraser le non-répertoire {$target} avec le répertoire {$source}
mv-error-not-directory = cible {$path} : N'est pas un répertoire
mv-error-target-not-directory = répertoire cible {$path} : N'est pas un répertoire
mv-error-failed-access-not-directory = impossible d'accéder à {$path} : N'est pas un répertoire
mv-error-backup-with-no-clobber = impossible de combiner --backup avec -n/--no-clobber ou --update=none-fail
mv-error-extra-operand = mv : opérande supplémentaire {$operand}
mv-error-backup-might-destroy-source = sauvegarder {$target} pourrait détruire la source ; {$source} non déplacé
mv-error-will-not-overwrite-just-created = ne va pas écraser le fichier qui vient d'être créé {$target} avec {$source}
mv-error-not-replacing = ne remplace pas {$target}
mv-error-cannot-move = impossible de déplacer {$source} vers {$target}
mv-error-directory-not-empty = Répertoire non vide
mv-error-dangling-symlink = impossible de déterminer le type de lien symbolique, car il est suspendu
mv-error-no-symlink-support = votre système d'exploitation ne prend pas en charge les liens symboliques
mv-error-permission-denied = Permission refusée
mv-error-inter-device-move-failed = échec du déplacement inter-périphérique : {$from} vers {$to} ; impossible de supprimer la cible : {$err}

# Messages d'aide
mv-help-force = ne pas demander avant d'écraser
mv-help-interactive = demander avant d'écraser
mv-help-no-clobber = ne pas écraser un fichier existant
mv-help-strip-trailing-slashes = supprimer toutes les barres obliques de fin de chaque argument SOURCE
mv-help-target-directory = déplacer tous les arguments SOURCE dans RÉPERTOIRE
mv-help-no-target-directory = traiter DEST comme un fichier normal
mv-help-verbose = expliquer ce qui est fait
mv-help-progress = Afficher une barre de progression.
  Note : cette fonctionnalité n'est pas supportée par GNU coreutils.
mv-help-debug = expliquer comment un fichier est copié. Implique -v

# Messages verbeux
mv-verbose-renamed = renommé {$from} -> {$to}
mv-verbose-renamed-with-backup = renommé {$from} -> {$to} (sauvegarde : {$backup})

# Messages de débogage
mv-debug-skipped = ignoré {$target}

# Messages de confirmation
mv-prompt-overwrite = écraser {$target} ?

# Messages de progression
mv-progress-moving = déplacement
