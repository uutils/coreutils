pathchk-about = Vérifier si les noms de fichiers sont valides ou portables
pathchk-usage = pathchk [OPTION]... NOM...

# Messages d'aide
pathchk-help-posix = vérifier pour la plupart des systèmes POSIX
pathchk-help-posix-special = vérifier les noms vides et les "-" en début
pathchk-help-portability = vérifier pour tous les systèmes POSIX (équivalent à -p -P)

# Messages d'erreur
pathchk-error-missing-operand = opérande manquant
pathchk-error-empty-file-name = nom de fichier vide
pathchk-error-posix-path-length-exceeded = limite { $limit } dépassée par la longueur { $length } du nom de fichier { $path }
pathchk-error-posix-name-length-exceeded = limite { $limit } dépassée par la longueur { $length } du composant de nom de fichier { $component }
pathchk-error-leading-hyphen = tiret en début dans le composant de nom de fichier { $component }
pathchk-error-path-length-exceeded = limite { $limit } dépassée par la longueur { $length } du nom de fichier { $path }
pathchk-error-name-length-exceeded = limite { $limit } dépassée par la longueur { $length } du composant de nom de fichier { $component }
pathchk-error-empty-path-not-found = pathchk: '' : Aucun fichier ou répertoire de ce type
pathchk-error-nonportable-character = caractère non portable '{ $character }' dans le composant de nom de fichier { $component }
