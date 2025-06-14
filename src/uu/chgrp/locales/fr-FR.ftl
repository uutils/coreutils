chgrp-about = Changer le groupe de chaque FICHIER vers GROUPE.
chgrp-usage = chgrp [OPTION]... GROUPE FICHIER...
  chgrp [OPTION]... --reference=RFICHIER FICHIER...

# Messages d'aide
chgrp-help-print-help = Afficher les informations d'aide.
chgrp-help-changes = comme verbeux mais rapporter seulement lors d'un changement
chgrp-help-quiet = supprimer la plupart des messages d'erreur
chgrp-help-verbose = afficher un diagnostic pour chaque fichier traité
chgrp-help-preserve-root = échouer à opérer récursivement sur '/'
chgrp-help-no-preserve-root = ne pas traiter '/' spécialement (par défaut)
chgrp-help-reference = utiliser le groupe de RFICHIER plutôt que spécifier les valeurs de GROUPE
chgrp-help-from = changer le groupe seulement si son groupe actuel correspond à GROUPE
chgrp-help-recursive = opérer sur les fichiers et répertoires récursivement

# Messages d'erreur
chgrp-error-invalid-group-id = identifiant de groupe invalide : '{ $gid_str }'
chgrp-error-invalid-group = groupe invalide : '{ $group }'
chgrp-error-failed-to-get-attributes = échec de l'obtention des attributs de { $file }
chgrp-error-invalid-user = utilisateur invalide : '{ $from_group }'
