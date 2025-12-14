chroot-about = Exécuter COMMANDE avec le répertoire racine défini à NOUVRACINE.
chroot-usage = chroot [OPTION]... NOUVRACINE [COMMANDE [ARG]...]

# Messages d'aide
chroot-help-groups = Liste de groupes séparés par des virgules vers lesquels basculer
chroot-help-userspec = Utilisateur et groupe séparés par deux-points vers lesquels basculer.
chroot-help-skip-chdir = Utiliser cette option pour ne pas changer le répertoire de travail vers / après avoir changé le répertoire racine vers nouvracine, c.-à-d., à l'intérieur du chroot.

# Messages d'erreur
chroot-error-skip-chdir-only-permitted = l'option --skip-chdir n'est autorisée que si NOUVRACINE est l'ancien '/'
chroot-error-cannot-enter = impossible de faire chroot vers { $dir } : { $err }
chroot-error-command-failed = échec de l'exécution de la commande { $cmd } : { $err }
chroot-error-command-not-found = échec de l'exécution de la commande { $cmd } : { $err }
chroot-error-groups-parsing-failed = échec de l'analyse de --groups
chroot-error-invalid-group = groupe invalide : { $group }
chroot-error-invalid-group-list = liste de groupes invalide : { $list }
chroot-error-missing-newroot = Opérande manquant : NOUVRACINE
  Essayez '{ $util_name } --help' pour plus d'informations.
chroot-error-no-group-specified = aucun groupe spécifié pour l'uid inconnu : { $uid }
chroot-error-no-such-user = utilisateur invalide
chroot-error-no-such-group = groupe invalide
chroot-error-no-such-directory = impossible de changer le répertoire racine vers { $dir } : aucun répertoire de ce type
chroot-error-set-gid-failed = impossible de définir le gid à { $gid } : { $err }
chroot-error-set-groups-failed = impossible de définir les groupes : { $err }
chroot-error-set-user-failed = impossible de définir l'utilisateur à { $user } : { $err }
