stat-about = afficher le statut du fichier ou du système de fichiers.
stat-usage = stat [OPTION]... FICHIER...
stat-after-help = Séquences de format valides pour les fichiers (sans `--file-system`) :

  -`%a` : droits d'accès en octal (note : drapeaux printf '#' et '0')
  -`%A` : droits d'accès en format lisible
  -`%b` : nombre de blocs alloués (voir %B)
  -`%B` : la taille en octets de chaque bloc rapporté par %b
  -`%C` : chaîne de contexte de sécurité SELinux
  -`%d` : numéro de périphérique en décimal
  -`%D` : numéro de périphérique en hexadécimal
  -`%f` : mode brut en hexadécimal
  -`%F` : type de fichier
  -`%g` : ID de groupe du propriétaire
  -`%G` : nom de groupe du propriétaire
  -`%h` : nombre de liens physiques
  -`%i` : numéro d'inode
  -`%m` : point de montage
  -`%n` : nom de fichier
  -`%N` : nom de fichier avec guillemets et déréférencement (suivi) si lien symbolique
  -`%o` : suggestion de taille optimale de transfert E/S
  -`%s` : taille totale, en octets
  -`%t` : type de périphérique majeur en hex, pour les fichiers spéciaux caractère/bloc
  -`%T` : type de périphérique mineur en hex, pour les fichiers spéciaux caractère/bloc
  -`%u` : ID utilisateur du propriétaire
  -`%U` : nom d'utilisateur du propriétaire
  -`%w` : heure de création du fichier, lisible ; - si inconnue
  -`%W` : heure de création du fichier, secondes depuis l'Époque ; 0 si inconnue
  -`%x` : heure du dernier accès, lisible
  -`%X` : heure du dernier accès, secondes depuis l'Époque
  -`%y` : heure de la dernière modification de données, lisible
  -`%Y` : heure de la dernière modification de données, secondes depuis l'Époque
  -`%z` : heure du dernier changement de statut, lisible
  -`%Z` : heure du dernier changement de statut, secondes depuis l'Époque

  Séquences de format valides pour les systèmes de fichiers :

  -`%a` : blocs libres disponibles pour les non-superutilisateurs
  -`%b` : blocs de données totaux dans le système de fichiers
  -`%c` : nœuds de fichiers totaux dans le système de fichiers
  -`%d` : nœuds de fichiers libres dans le système de fichiers
  -`%f` : blocs libres dans le système de fichiers
  -`%i` : ID du système de fichiers en hexadécimal
  -`%l` : longueur maximale des noms de fichiers
  -`%n` : nom de fichier
  -`%s` : taille de bloc (pour des transferts plus rapides)
  -`%S` : taille de bloc fondamentale (pour les comptes de blocs)
  -`%t` : type de système de fichiers en hexadécimal
  -`%T` : type de système de fichiers en format lisible

  NOTE : votre shell peut avoir sa propre version de stat, qui remplace généralement
  la version décrite ici. Veuillez vous référer à la documentation de votre shell
  pour les détails sur les options qu'il prend en charge.

# Messages d'aide

stat-help-dereference = suivre les liens
stat-help-file-system = afficher le statut du système de fichiers au lieu du statut du fichier
stat-help-terse = afficher les informations en forme concise
stat-help-format = utiliser le FORMAT spécifié au lieu du défaut ;
 afficher une nouvelle ligne après chaque utilisation de FORMAT
stat-help-printf = comme --format, mais interpréter les séquences d'échappement avec barre oblique inverse,
  et ne pas afficher une nouvelle ligne finale obligatoire ;
  si vous voulez une nouvelle ligne, incluez \n dans FORMAT

## Traductions de mots

stat-word-file = Fichier
stat-word-id = ID
stat-word-namelen = Longnom
stat-word-type = Type
stat-word-block = Bloc
stat-word-size = taille
stat-word-fundamental = Fondamentale
stat-word-block-size = taille bloc
stat-word-blocks = Blocs
stat-word-total = Total
stat-word-free = Libres
stat-word-available = Disponibles
stat-word-inodes = Inodes
stat-word-device = Périphérique
stat-word-inode = Inode
stat-word-links = Liens
stat-word-io = E/S
stat-word-access = Accès
stat-word-uid = Uid
stat-word-gid = Gid
stat-word-modify = Modif
stat-word-change = Changt
stat-word-birth = Créé

## Messages d'erreur

stat-error-invalid-quoting-style = Style de guillemets invalide : {$style}
stat-error-missing-operand = opérande manquant
  Essayez 'stat --help' pour plus d'informations.
stat-error-invalid-directive = {$directive} : directive invalide
stat-error-cannot-read-filesystem = impossible de lire la table des systèmes de fichiers montés : {$error}
stat-error-stdin-filesystem-mode = utiliser '-' pour désigner l'entrée standard ne fonctionne pas en mode système de fichiers
stat-error-cannot-read-filesystem-info = impossible de lire les informations du système de fichiers pour {$file} : {$error}
stat-error-cannot-stat = impossible d'obtenir le statut de {$file} : {$error}

## Messages d'avertissement

stat-warning-backslash-end-format = barre oblique inverse à la fin du format
stat-warning-unrecognized-escape-x = séquence d'échappement non reconnue '\x'
stat-warning-incomplete-hex-escape = séquence d'échappement hexadécimale incomplète '\x'
stat-warning-unrecognized-escape = séquence d'échappement non reconnue '\{$escape}'

## Messages de contexte SELinux

stat-selinux-failed-get-context = impossible d'obtenir le contexte de sécurité
stat-selinux-unsupported-system = non pris en charge sur ce système
stat-selinux-unsupported-os = non pris en charge pour ce système d'exploitation
