mknod-about = Créer le fichier spécial NOM du TYPE donné.
mknod-usage = mknod [OPTION]... NOM TYPE [MAJEUR MINEUR]
mknod-after-help = Les arguments obligatoires pour les options longues le sont aussi pour les options courtes.
  -m, --mode=MODE définir les bits de permission de fichier à MODE, pas a=rw - umask

  MAJEUR et MINEUR doivent tous deux être spécifiés quand TYPE est b, c, ou u, et ils
  doivent être omis quand TYPE est p. Si MAJEUR ou MINEUR commence par 0x ou 0X,
  il est interprété comme hexadécimal ; sinon, s'il commence par 0, comme octal ;
  sinon, comme décimal. TYPE peut être :

  - b créer un fichier spécial bloc (mis en mémoire tampon)
  - c, u créer un fichier spécial caractère (non mis en mémoire tampon)
  - p créer un FIFO

  NOTE : votre shell peut avoir sa propre version de mknod, qui remplace généralement
  la version décrite ici. Veuillez vous référer à la documentation de votre shell
  pour les détails sur les options qu'il supporte.

# Messages d'aide
mknod-help-mode = définir les bits de permission de fichier à MODE, pas a=rw - umask
mknod-help-name = nom du nouveau fichier
mknod-help-type = type du nouveau fichier (b, c, u ou p)
mknod-help-major = type de fichier majeur
mknod-help-minor = type de fichier mineur
mknod-help-selinux = définir le contexte de sécurité SELinux de chaque répertoire créé au type par défaut
mknod-help-context = comme -Z, ou si CTX est spécifié, définir le contexte de sécurité SELinux ou SMACK à CTX

# Messages d'erreur
mknod-error-fifo-no-major-minor = Les fifos n'ont pas de numéros de périphérique majeur et mineur.
mknod-error-special-require-major-minor = Les fichiers spéciaux nécessitent des numéros de périphérique majeur et mineur.
mknod-error-invalid-mode = mode invalide ({ $error })
mknod-error-mode-permission-bits-only = le mode ne doit spécifier que les bits de permission de fichier
mknod-error-missing-device-type = type de périphérique manquant
mknod-error-invalid-device-type = type de périphérique invalide { $type }
