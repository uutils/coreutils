chown-about = Changer le propriétaire et le groupe des fichiers
chown-usage = chown [OPTION]... [PROPRIÉTAIRE][:[GROUPE]] FICHIER...
  chown [OPTION]... --reference=RFICHIER FICHIER...

# Messages d'aide
chown-help-print-help = Afficher les informations d'aide.
chown-help-changes = comme verbeux mais rapporter seulement lors d'un changement
chown-help-from = changer le propriétaire et/ou le groupe de chaque fichier seulement si son
  propriétaire et/ou groupe actuel correspondent à ceux spécifiés ici.
  L'un ou l'autre peut être omis, auquel cas une correspondance n'est pas requise
  pour l'attribut omis
chown-help-preserve-root = échouer à opérer récursivement sur '/'
chown-help-no-preserve-root = ne pas traiter '/' spécialement (par défaut)
chown-help-quiet = supprimer la plupart des messages d'erreur
chown-help-recursive = opérer sur les fichiers et répertoires récursivement
chown-help-reference = utiliser le propriétaire et groupe de RFICHIER plutôt que spécifier les valeurs PROPRIÉTAIRE:GROUPE
chown-help-verbose = afficher un diagnostic pour chaque fichier traité

# Messages d'erreur
chown-error-failed-to-get-attributes = échec de l'obtention des attributs de { $file }
chown-error-invalid-user = utilisateur invalide : { $user }
chown-error-invalid-group = groupe invalide : { $group }
chown-error-invalid-spec = spécification invalide : { $spec }
