mkdir-about = Créer les RÉPERTOIRE(s) donnés s'ils n'existent pas
mkdir-usage = mkdir [OPTION]... RÉPERTOIRE...
mkdir-after-help = Chaque MODE est de la forme [ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+.

# Messages d'aide
mkdir-help-mode = définir le mode de fichier (non implémenté sur Windows)
mkdir-help-parents = créer les répertoires parents si nécessaire
mkdir-help-verbose = afficher un message pour chaque répertoire créé
mkdir-help-selinux = définir le contexte de sécurité SELinux de chaque répertoire créé au type par défaut
mkdir-help-context = comme -Z, ou si CTX est spécifié, définir le contexte de sécurité SELinux ou SMACK à CTX

# Messages d'erreur
mkdir-error-empty-directory-name = impossible de créer le répertoire '' : Aucun fichier ou répertoire de ce type
mkdir-error-file-exists = { $path } : Le fichier existe
mkdir-error-failed-to-create-tree = échec de la création de l'arborescence complète
mkdir-error-cannot-set-permissions = impossible de définir les permissions { $path }

# Sortie détaillée
mkdir-verbose-created-directory = { $util_name } : répertoire créé { $path }
