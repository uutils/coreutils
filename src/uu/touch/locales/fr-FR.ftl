
touch-about = Mettre à jour les temps d'accès et de modification de chaque FICHIER avec l'heure actuelle.
touch-usage = touch [OPTION]... [FICHIER]...

# Messages d'aide
touch-help-help = Afficher les informations d'aide.
touch-help-access = changer seulement le temps d'accès
touch-help-timestamp = utiliser [[CC]AA]MMJJhhmm[.ss] au lieu de l'heure actuelle
touch-help-date = analyser l'argument et l'utiliser au lieu de l'heure actuelle
touch-help-modification = changer seulement le temps de modification
touch-help-no-create = ne créer aucun fichier
touch-help-no-deref = affecter chaque lien symbolique au lieu de tout fichier référencé (seulement pour les systèmes qui peuvent changer les horodatages d'un lien symbolique)
touch-help-reference = utiliser les temps de ce fichier au lieu de l'heure actuelle
touch-help-time = changer seulement le temps spécifié : "access", "atime", ou "use" sont équivalents à -a ; "modify" ou "mtime" sont équivalents à -m

# Messages d'erreur
touch-error-missing-file-operand = opérande de fichier manquant
  Essayez '{ $help_command } --help' pour plus d'informations.
touch-error-setting-times-of = définition des temps de { $filename }
touch-error-setting-times-no-such-file = définition des temps de { $filename } : Aucun fichier ou répertoire de ce type
touch-error-cannot-touch = impossible de toucher { $filename }
touch-error-no-such-file-or-directory = Aucun fichier ou répertoire de ce type
touch-error-failed-to-get-attributes = échec d'obtention des attributs de { $path }
touch-error-setting-times-of-path = définition des temps de { $path }
touch-error-invalid-date-ts-format = format de date ts invalide { $date }
touch-error-invalid-date-format = format de date invalide { $date }
touch-error-unable-to-parse-date = Impossible d'analyser la date : { $date }
touch-error-windows-stdout-path-failed = GetFinalPathNameByHandleW a échoué avec le code { $code }
touch-error-invalid-filetime = La source a un temps d'accès ou de modification invalide : { $time }
touch-error-reference-file-inaccessible = échec d'obtention des attributs de { $path } : { $error }
