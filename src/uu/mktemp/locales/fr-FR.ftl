mktemp-about = Créer un fichier ou répertoire temporaire.
mktemp-usage = mktemp [OPTION]... [MODÈLE]

# Messages d'aide
mktemp-help-directory = Créer un répertoire au lieu d'un fichier
mktemp-help-dry-run = ne rien créer ; afficher seulement un nom (dangereux)
mktemp-help-quiet = Échouer silencieusement si une erreur se produit.
mktemp-help-suffix = ajouter SUFFIXE au MODÈLE ; SUFFIXE ne doit pas contenir un séparateur de chemin. Cette option est implicite si MODÈLE ne se termine pas par X.
mktemp-help-p = forme courte de --tmpdir
mktemp-help-tmpdir = interpréter MODÈLE relativement à RÉP ; si RÉP n'est pas spécifié, utiliser $TMPDIR ($TMP sur windows) si défini, sinon /tmp. Avec cette option, MODÈLE ne doit pas être un nom absolu ; contrairement à -t, MODÈLE peut contenir des barres obliques, mais mktemp ne crée que le composant final
mktemp-help-t = Générer un modèle (en utilisant le préfixe fourni et TMPDIR (TMP sur windows) si défini) pour créer un modèle de nom de fichier [obsolète]

# Messages d'erreur
mktemp-error-persist-file = impossible de conserver le fichier { $path }
mktemp-error-must-end-in-x = avec --suffix, le modèle { $template } doit se terminer par X
mktemp-error-too-few-xs = trop peu de X dans le modèle { $template }
mktemp-error-prefix-contains-separator = modèle invalide, { $template }, contient un séparateur de répertoire
mktemp-error-suffix-contains-separator = suffixe invalide { $suffix }, contient un séparateur de répertoire
mktemp-error-invalid-template = modèle invalide, { $template } ; avec --tmpdir, il ne peut pas être absolu
mktemp-error-too-many-templates = trop de modèles
mktemp-error-not-found = échec de la création de { $template_type } via le modèle { $template } : Aucun fichier ou répertoire de ce type
mktemp-error-failed-print = échec de l'affichage du nom de répertoire

# Types de modèle
mktemp-template-type-directory = répertoire
mktemp-template-type-file = fichier
