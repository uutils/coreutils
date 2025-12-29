ln-about = Créer des liens entre fichiers
ln-usage = ln [OPTION]... [-T] CIBLE NOM_LIEN
  ln [OPTION]... CIBLE
  ln [OPTION]... CIBLE... RÉPERTOIRE
  ln [OPTION]... -t RÉPERTOIRE CIBLE...
ln-after-help = Dans la 1ère forme, créer un lien vers CIBLE avec le nom NOM_LIEN.
  Dans la 2ème forme, créer un lien vers CIBLE dans le répertoire courant.
  Dans les 3ème et 4ème formes, créer des liens vers chaque CIBLE dans RÉPERTOIRE.
  Créer des liens physiques par défaut, des liens symboliques avec --symbolic.
  Par défaut, chaque destination (nom du nouveau lien) ne doit pas déjà exister.
  Lors de la création de liens physiques, chaque CIBLE doit exister. Les liens symboliques
  peuvent contenir du texte arbitraire ; s'ils sont résolus plus tard, un lien relatif est
  interprété en relation avec son répertoire parent.

ln-help-force = supprimer les fichiers de destination existants
ln-help-interactive = demander avant de supprimer les fichiers de destination existants
ln-help-no-dereference = traiter NOM_LIEN comme un fichier normal s'il s'agit d'un
                          lien symbolique vers un répertoire
ln-help-logical = suivre les CIBLEs qui sont des liens symboliques
ln-help-physical = créer des liens physiques directement vers les liens symboliques
ln-help-symbolic = créer des liens symboliques au lieu de liens physiques
ln-help-target-directory = spécifier le RÉPERTOIRE dans lequel créer les liens
ln-help-no-target-directory = toujours traiter NOM_LIEN comme un fichier normal
ln-help-relative = créer des liens symboliques relatifs à l'emplacement du lien
ln-help-verbose = afficher le nom de chaque fichier lié

ln-error-target-is-not-directory = la cible {$target} n'est pas un répertoire
ln-error-same-file = {$file1} et {$file2} sont le même fichier
ln-error-missing-destination = opérande de fichier de destination manquant après {$operand}
ln-error-extra-operand = opérande supplémentaire {$operand}
  Essayez « {$program} --help » pour plus d'informations.
ln-error-could-not-update = Impossible de mettre à jour {$target} : {$error}
ln-error-cannot-stat = impossible d'analyser {$path} : Aucun fichier ou répertoire de ce nom
ln-error-will-not-overwrite = ne remplacera pas le fichier {$target} qui vient d'être créé par {$source}
ln-prompt-replace = remplacer {$file} ?
ln-cannot-backup = impossible de sauvegarder {$file}
ln-failed-to-access = échec d'accès à {$file}
ln-failed-to-create-hard-link = échec de création du lien physique {$source} => {$dest}
ln-failed-to-create-hard-link-dir = {$source} : lien physique non autorisé pour un répertoire
ln-backup = sauvegarde : {$backup}
