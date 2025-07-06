mkfifo-about = Créer un FIFO avec le nom donné.
mkfifo-usage = mkfifo [OPTION]... NOM...

# Messages d'aide
mkfifo-help-mode = permissions de fichier pour le fifo
mkfifo-help-selinux = définir le contexte de sécurité SELinux au type par défaut
mkfifo-help-context = comme -Z, ou si CTX est spécifié, définir le contexte de sécurité SELinux ou SMACK à CTX

# Messages d'erreur
mkfifo-error-invalid-mode = mode invalide : { $error }
mkfifo-error-missing-operand = opérande manquant
mkfifo-error-cannot-create-fifo = impossible de créer le fifo { $path } : Le fichier existe
mkfifo-error-cannot-set-permissions = impossible de définir les permissions sur { $path } : { $error }
