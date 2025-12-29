tail-about = Afficher les 10 dernières lignes de chaque FICHIER sur la sortie standard.
  Avec plus d'un FICHIER, précéder chacun d'un en-tête donnant le nom du fichier.
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.
  Les arguments obligatoires pour les drapeaux longs sont également obligatoires pour les drapeaux courts.
tail-usage = tail [DRAPEAU]... [FICHIER]...

# Messages d'aide
tail-help-bytes = Nombre d'octets à afficher
tail-help-follow = Afficher le fichier au fur et à mesure de sa croissance
tail-help-lines = Nombre de lignes à afficher
tail-help-pid = Avec -f, terminer après que l'ID de processus, PID meure
tail-help-quiet = Ne jamais afficher d'en-têtes donnant les noms de fichiers
tail-help-sleep-interval = Nombre de secondes à attendre entre les sondages du fichier lors de l'exécution avec -f
tail-help-max-unchanged-stats = Rouvrir un FICHIER qui n'a pas changé de taille après N (par défaut 5) itérations pour voir s'il a été supprimé ou renommé (c'est le cas habituel des fichiers journaux pivotés) ; Cette option n'a de sens que lors du sondage (c'est-à-dire avec --use-polling) et quand --follow=name
tail-help-verbose = Toujours afficher des en-têtes donnant les noms de fichiers
tail-help-zero-terminated = Le délimiteur de ligne est NUL, pas newline
tail-help-retry = Continuer d'essayer d'ouvrir un fichier s'il est inaccessible
tail-help-follow-retry = Identique à --follow=name --retry
tail-help-polling-linux = Désactiver le support 'inotify' et utiliser le sondage à la place
tail-help-polling-unix = Désactiver le support 'kqueue' et utiliser le sondage à la place
tail-help-polling-windows = Désactiver le support 'ReadDirectoryChanges' et utiliser le sondage à la place

# Messages d'erreur
tail-error-cannot-follow-stdin-by-name = impossible de suivre { $stdin } par nom
tail-error-cannot-open-no-such-file = impossible d'ouvrir '{ $file }' en lecture : { $error }
tail-error-reading-file = erreur de lecture de '{ $file }' : { $error }
tail-error-cannot-follow-file-type = { $file } : impossible de suivre la fin de ce type de fichier{ $msg }
tail-error-cannot-open-for-reading = impossible d'ouvrir '{ $file }' en lecture
tail-error-cannot-fstat = impossible de faire fstat { $file } : { $error }
tail-error-invalid-number-of-bytes = nombre d'octets invalide : { $arg }
tail-error-invalid-number-of-lines = nombre de lignes invalide : { $arg }
tail-error-invalid-number-of-seconds = nombre de secondes invalide : '{ $source }'
tail-error-invalid-max-unchanged-stats = nombre maximum invalide de statistiques inchangées entre les ouvertures : { $value }
tail-error-invalid-pid = PID invalide : { $pid }
tail-error-invalid-pid-with-error = PID invalide : { $pid } : { $error }
tail-error-invalid-number-out-of-range = nombre invalide : { $arg } : Résultat numérique hors limites
tail-error-invalid-number-overflow = nombre invalide : { $arg }
tail-error-option-used-in-invalid-context = option utilisée dans un contexte invalide -- { $option }
tail-error-bad-argument-encoding = encodage d'argument incorrect : { $arg }
tail-error-cannot-watch-parent-directory = impossible de surveiller le répertoire parent de { $path }
tail-error-backend-cannot-be-used-too-many-files = { $backend } ne peut pas être utilisé, retour au sondage : Trop de fichiers ouverts
tail-error-backend-resources-exhausted = ressources { $backend } épuisées
tail-error-notify-error = Erreur de notification : { $error }
tail-error-recv-timeout-error = Erreur de délai de réception : { $error }

# Messages d'avertissement
tail-warning-retry-ignored = --retry ignoré ; --retry n'est utile que lors du suivi
tail-warning-retry-only-effective = --retry n'est effectif que pour l'ouverture initiale
tail-warning-pid-ignored = PID ignoré ; --pid=PID n'est utile que lors du suivi
tail-warning-pid-not-supported = --pid=PID n'est pas pris en charge sur ce système
tail-warning-following-stdin-ineffective = suivre l'entrée standard indéfiniment est inefficace

# Messages de statut
tail-status-has-become-accessible = { $file } est devenu accessible
tail-status-has-appeared-following-new-file = { $file } est apparu ; suivi du nouveau fichier
tail-status-has-been-replaced-following-new-file = { $file } a été remplacé ; suivi du nouveau fichier
tail-status-file-truncated = { $file } : fichier tronqué
tail-status-replaced-with-untailable-file = { $file } a été remplacé par un fichier non suivable
tail-status-replaced-with-untailable-file-giving-up = { $file } a été remplacé par un fichier non suivable ; abandon de ce nom
tail-status-file-became-inaccessible = { $file } { $become_inaccessible } : { $no_such_file }
tail-status-directory-containing-watched-file-removed = le répertoire contenant le fichier surveillé a été supprimé
tail-status-backend-cannot-be-used-reverting-to-polling = { $backend } ne peut pas être utilisé, retour au sondage
tail-status-file-no-such-file = { $file } : { $no_such_file }

# Constantes de texte
tail-bad-fd = Descripteur de fichier incorrect
tail-no-such-file-or-directory = Aucun fichier ou répertoire de ce type
tail-is-a-directory = Est un répertoire
tail-giving-up-on-this-name = ; abandon de ce nom
tail-stdin-header = entrée standard
tail-no-files-remaining = aucun fichier restant
tail-become-inaccessible = est devenu inaccessible
