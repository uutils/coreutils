rm-about = Supprimer (délier) le(s) FICHIER(s)
rm-usage = rm [OPTION]... FICHIER...
rm-after-help = Par défaut, rm ne supprime pas les répertoires. Utilisez l'option --recursive (-r ou -R)
  pour supprimer également chaque répertoire listé, ainsi que tout son contenu

  Pour supprimer un fichier dont le nom commence par un '-', par exemple '-foo',
  utilisez une de ces commandes :
  rm -- -foo

  rm ./-foo

  Notez que si vous utilisez rm pour supprimer un fichier, il pourrait être possible de récupérer
  une partie de son contenu, avec suffisamment d'expertise et/ou de temps. Pour une meilleure
  assurance que le contenu est vraiment irrécupérable, considérez utiliser shred.

# Texte d'aide pour les options
rm-help-force = ignorer les fichiers inexistants et les arguments, ne jamais demander
rm-help-prompt-always = demander avant chaque suppression
rm-help-prompt-once = demander une fois avant de supprimer plus de trois fichiers, ou lors d'une suppression récursive.
  Moins intrusif que -i, tout en offrant une protection contre la plupart des erreurs
rm-help-interactive = demander selon QUAND : never, once (-I), ou always (-i). Sans QUAND,
  demande toujours
rm-help-one-file-system = lors de la suppression récursive d'une hiérarchie, ignorer tout répertoire situé sur un
  système de fichiers différent de celui de l'argument de ligne de commande correspondant (NON
  IMPLÉMENTÉ)
rm-help-no-preserve-root = ne pas traiter '/' spécialement
rm-help-preserve-root = ne pas supprimer '/' (par défaut)
rm-help-recursive = supprimer les répertoires et leur contenu récursivement
rm-help-dir = supprimer les répertoires vides
rm-help-verbose = expliquer ce qui est fait
rm-help-progress = afficher une barre de progression. Note : cette fonctionnalité n'est pas supportée par GNU coreutils.

# Messages de progression
rm-progress-removing = Suppression

# Messages d'erreur
rm-error-missing-operand = opérande manquant
  Essayez '{$util_name} --help' pour plus d'informations.
rm-error-cannot-remove-no-such-file = impossible de supprimer {$file} : Aucun fichier ou répertoire de ce type
rm-error-cannot-remove-permission-denied = impossible de supprimer {$file} : Permission refusée
rm-error-cannot-remove-is-directory = impossible de supprimer {$file} : C'est un répertoire
rm-error-dangerous-recursive-operation = il est dangereux d'opérer récursivement sur '/'
rm-error-use-no-preserve-root = utilisez --no-preserve-root pour outrepasser cette protection
rm-error-refusing-to-remove-directory = refus de supprimer le répertoire '.' ou '..' : ignorer {$path}
rm-error-cannot-remove = impossible de supprimer {$file}

# Messages verbeux
rm-verbose-removed = {$file} supprimé
rm-verbose-removed-directory = répertoire {$file} supprimé
