shred-about = Écrase les FICHIER(s) spécifiés de manière répétée, afin de rendre plus difficile
  même pour du matériel de sondage très coûteux de récupérer les données.
shred-usage = shred [OPTION]... FICHIER...
shred-after-help = Supprime le ou les FICHIER(s) si --remove (-u) est spécifié. Par défaut, les fichiers
  ne sont pas supprimés car il est courant d'opérer sur des fichiers de périphérique comme /dev/hda,
  et ces fichiers ne doivent généralement pas être supprimés.

  ATTENTION : Notez que shred repose sur une hypothèse très importante : que le système
  de fichiers écrase les données sur place. C'est la façon traditionnelle de procéder, mais
  de nombreuses conceptions de systèmes de fichiers modernes ne satisfont pas cette hypothèse.
  Voici des exemples de systèmes de fichiers sur lesquels shred n'est pas efficace, ou n'est pas
  garanti d'être efficace dans tous les modes de système de fichiers :

   - systèmes de fichiers structurés en journal ou en log, tels que ceux fournis avec
     AIX et Solaris (et JFS, ReiserFS, XFS, Ext3, etc.)

   - systèmes de fichiers qui écrivent des données redondantes et continuent même si certaines écritures
     échouent, tels que les systèmes de fichiers basés sur RAID

   - systèmes de fichiers qui font des instantanés, tels que le serveur NFS de Network Appliance

   - systèmes de fichiers qui mettent en cache dans des emplacements temporaires, tels que NFS
     version 3 clients

   - systèmes de fichiers compressés

  Dans le cas des systèmes de fichiers ext3, la clause de non-responsabilité ci-dessus s'applique (et shred est
  donc d'efficacité limitée) seulement en mode data=journal, qui journalise les données de fichier
  en plus des métadonnées seulement. Dans les modes data=ordered (par défaut) et
  data=writeback, shred fonctionne comme d'habitude. Les modes de journal Ext3 peuvent être changés
  en ajoutant l'option data=something aux options de montage pour un système de fichiers particulier
  dans le fichier /etc/fstab, comme documenté dans la page de manuel mount (`man mount`).

  De plus, les sauvegardes de système de fichiers et les miroirs distants peuvent contenir des copies
  du fichier qui ne peuvent pas être supprimées, et qui permettront à un fichier détruit d'être
  récupéré plus tard.

# Messages d'erreur
shred-missing-file-operand = opérande de fichier manquant
shred-invalid-number-of-passes = nombre de passes invalide : {$passes}
shred-cannot-open-random-source = impossible d'ouvrir la source aléatoire : {$source}
shred-invalid-file-size = taille de fichier invalide : {$size}
shred-no-such-file-or-directory = {$file} : Aucun fichier ou répertoire de ce type
shred-not-a-file = {$file} : N'est pas un fichier

# Texte d'aide des options
shred-force-help = modifier les permissions pour permettre l'écriture si nécessaire
shred-iterations-help = écraser N fois au lieu de la valeur par défaut (3)
shred-size-help = détruire ce nombre d'octets (suffixes comme K, M, G acceptés)
shred-deallocate-help = désallouer et supprimer le fichier après écrasement
shred-remove-help = comme -u mais donne le contrôle sur COMMENT supprimer ; Voir ci-dessous
shred-verbose-help = afficher le progrès
shred-exact-help = ne pas arrondir les tailles de fichier au bloc complet suivant ;
                   c'est la valeur par défaut pour les fichiers non réguliers
shred-zero-help = ajouter un écrasement final avec des zéros pour cacher la destruction
shred-random-source-help = prendre des octets aléatoires du FICHIER

# Messages verbeux
shred-removing = {$file} : suppression
shred-removed = {$file} : supprimé
shred-renamed-to = renommé en
shred-pass-progress = {$file}: passage
shred-couldnt-rename = {$file} : Impossible de renommer en {$new_name} : {$error}
shred-failed-to-open-for-writing = {$file} : impossible d'ouvrir pour l'écriture
shred-file-write-pass-failed = {$file} : Échec du passage d'écriture de fichier
shred-failed-to-remove-file = {$file} : impossible de supprimer le fichier

# Messages d'erreur E/S de fichier
shred-failed-to-clone-file-handle = échec du clonage du descripteur de fichier
shred-failed-to-seek-file = échec de la recherche dans le fichier
shred-failed-to-read-seed-bytes = échec de la lecture des octets de graine du fichier
shred-failed-to-get-metadata = échec de l'obtention des métadonnées du fichier
shred-failed-to-set-permissions = échec de la définition des permissions du fichier
