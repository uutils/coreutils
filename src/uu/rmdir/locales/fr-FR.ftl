rmdir-about = Supprimer les RÉPERTOIRE(S), s'ils sont vides.
rmdir-usage = rmdir [OPTION]... RÉPERTOIRE...

# Messages d'aide
rmdir-help-ignore-fail-non-empty = ignorer chaque échec qui est uniquement dû au fait qu'un répertoire n'est pas vide
rmdir-help-parents = supprimer RÉPERTOIRE et ses ancêtres ; p. ex., 'rmdir -p a/b/c' est similaire à rmdir a/b/c a/b a
rmdir-help-verbose = afficher un diagnostic pour chaque répertoire traité

# Messages d'erreur
rmdir-error-symbolic-link-not-followed = échec de la suppression de { $path } : Lien symbolique non suivi
rmdir-error-failed-to-remove = échec de la suppression de { $path } : { $err }

# Sortie détaillée
rmdir-verbose-removing-directory = { $util_name } : suppression du répertoire, { $path }
