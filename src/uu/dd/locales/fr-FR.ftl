dd-about = Copier, et optionnellement convertir, une ressource du système de fichiers
dd-usage = dd [OPÉRANDE]...
  dd OPTION
dd-after-help = ### Opérandes

  - bs=OCTETS : lire et écrire jusqu'à OCTETS octets à la fois (par défaut : 512) ;
     remplace ibs et obs.
  - cbs=OCTETS : la 'taille de bloc de conversion' en octets. S'applique aux
     opérations conv=block et conv=unblock.
  - conv=CONVS : une liste séparée par des virgules d'options de conversion ou (pour des
     raisons historiques) d'indicateurs de fichier.
  - count=N : arrêter la lecture de l'entrée après N opérations de lecture de taille ibs
     plutôt que de continuer jusqu'à EOF. Voir iflag=count_bytes si l'arrêt après N octets
     est préféré
  - ibs=N : la taille du tampon utilisé pour les lectures (par défaut : 512)
  - if=FICHIER : le fichier utilisé pour l'entrée. Quand non spécifié, stdin est utilisé à la place
  - iflag=INDICATEURS : une liste séparée par des virgules d'indicateurs d'entrée qui spécifient comment
     la source d'entrée est traitée. INDICATEURS peut être n'importe lequel des indicateurs d'entrée ou
     indicateurs généraux spécifiés ci-dessous.
  - skip=N (ou iseek=N) : ignorer N enregistrements de taille ibs dans l'entrée avant de commencer
     les opérations de copie/conversion. Voir iflag=seek_bytes si la recherche de N octets est préférée.
  - obs=N : la taille du tampon utilisé pour les écritures (par défaut : 512)
  - of=FICHIER : le fichier utilisé pour la sortie. Quand non spécifié, stdout est utilisé
     à la place
  - oflag=INDICATEURS : liste séparée par des virgules d'indicateurs de sortie qui spécifient comment la
     source de sortie est traitée. INDICATEURS peut être n'importe lequel des indicateurs de sortie ou
     indicateurs généraux spécifiés ci-dessous
  - seek=N (ou oseek=N) : recherche N enregistrements de taille obs dans la sortie avant de
     commencer les opérations de copie/conversion. Voir oflag=seek_bytes si la recherche de N octets est
     préférée
  - status=NIVEAU : contrôle si les statistiques de volume et de performance sont écrites sur
     stderr.

    Quand non spécifié, dd affichera les statistiques à la fin. Un exemple est ci-dessous.

    ```plain
      6+0 enregistrements en entrée
      16+0 enregistrements en sortie
      8192 octets (8.2 kB, 8.0 KiB) copiés, 0.00057009 s,
      14.4 MB/s

    Les deux premières lignes sont les statistiques de 'volume' et la dernière ligne est les
    statistiques de 'performance'.
    Les statistiques de volume indiquent le nombre de lectures complètes et partielles de taille ibs,
    ou d'écritures de taille obs qui ont eu lieu pendant la copie. Le format des statistiques de
    volume est <complètes>+<partielles>. Si des enregistrements ont été tronqués (voir
    conv=block), les statistiques de volume contiendront le nombre d'enregistrements tronqués.

    Les valeurs possibles de NIVEAU sont :
    - progress : Afficher les statistiques de performance périodiques pendant la copie.
    - noxfer : Afficher les statistiques de volume finales, mais pas les statistiques de performance.
    - none : N'afficher aucune statistique.

    L'affichage des statistiques de performance est aussi déclenché par le signal INFO (quand supporté),
    ou le signal USR1. Définir la variable d'environnement POSIXLY_CORRECT à n'importe quelle valeur
    (y compris une valeur vide) fera ignorer le signal USR1.

  ### Options de conversion

  - ascii : convertir d'EBCDIC vers ASCII. C'est l'inverse de l'option ebcdic.
    Implique conv=unblock.
  - ebcdic : convertir d'ASCII vers EBCDIC. C'est l'inverse de l'option ascii.
    Implique conv=block.
  - ibm : convertir d'ASCII vers EBCDIC, en appliquant les conventions pour [, ]
    et ~ spécifiées dans POSIX. Implique conv=block.

  - ucase : convertir de minuscules vers majuscules.
  - lcase : convertir de majuscules vers minuscules.

  - block : pour chaque nouvelle ligne inférieure à la taille indiquée par cbs=OCTETS, supprimer
    la nouvelle ligne et remplir avec des espaces jusqu'à cbs. Les lignes plus longues que cbs sont tronquées.
  - unblock : pour chaque bloc d'entrée de la taille indiquée par cbs=OCTETS, supprimer
    les espaces de fin à droite et remplacer par un caractère de nouvelle ligne.

  - sparse : tente de rechercher la sortie quand un bloc de taille obs ne contient que
    des zéros.
  - swab : échange chaque paire d'octets adjacents. Si un nombre impair d'octets est
    présent, l'octet final est omis.
  - sync : remplit chaque bloc de taille ibs avec des zéros. Si block ou unblock est
    spécifié, remplit avec des espaces à la place.
  - excl : le fichier de sortie doit être créé. Échoue si le fichier de sortie est déjà
    présent.
  - nocreat : le fichier de sortie ne sera pas créé. Échoue si le fichier de sortie n'est
    pas déjà présent.
  - notrunc : le fichier de sortie ne sera pas tronqué. Si cette option n'est pas
    présente, la sortie sera tronquée à l'ouverture.
  - noerror : toutes les erreurs de lecture seront ignorées. Si cette option n'est pas présente,
    dd n'ignorera que Error::Interrupted.
  - fdatasync : les données seront écrites avant la fin.
  - fsync : les données et les métadonnées seront écrites avant la fin.

  ### Indicateurs d'entrée

  - count_bytes : une valeur pour count=N sera interprétée comme des octets.
  - skip_bytes : une valeur pour skip=N sera interprétée comme des octets.
  - fullblock : attendre ibs octets de chaque lecture. les lectures de longueur zéro sont toujours
    considérées comme EOF.

  ### Indicateurs de sortie

  - append : ouvrir le fichier en mode ajout. Considérez définir conv=notrunc aussi.
  - seek_bytes : une valeur pour seek=N sera interprétée comme des octets.

  ### Indicateurs généraux

  - direct : utiliser les E/S directes pour les données.
  - directory : échouer sauf si l'entrée donnée (si utilisée comme iflag) ou
    la sortie (si utilisée comme oflag) est un répertoire.
  - dsync : utiliser les E/S synchronisées pour les données.
  - sync : utiliser les E/S synchronisées pour les données et les métadonnées.
  - nonblock : utiliser les E/S non-bloquantes.
  - noatime : ne pas mettre à jour l'heure d'accès.
  - nocache : demander au système d'exploitation de supprimer le cache.
  - noctty : ne pas assigner un tty de contrôle.
  - nofollow : ne pas suivre les liens système.

# Common strings
dd-standard-input = 'entrée standard'
dd-standard-output = 'sortie standard'

# Error messages
dd-error-failed-to-open = échec de l'ouverture de { $path }
dd-error-write-error = erreur d'écriture
dd-error-failed-to-seek = échec de la recherche dans le fichier de sortie
dd-error-io-error = erreur E/S
dd-error-cannot-skip-offset = '{ $file }' : impossible d'ignorer jusqu'au décalage spécifié
dd-error-cannot-skip-invalid = '{ $file }' : impossible d'ignorer : Argument invalide
dd-error-cannot-seek-invalid = '{ $output }' : impossible de rechercher : Argument invalide
dd-error-not-directory = définir les indicateurs pour '{ $file }' : N'est pas un répertoire
dd-error-failed-discard-cache = échec de la suppression du cache pour : { $file }

# Parse errors
dd-error-unrecognized-operand = Opérande non reconnue '{ $operand }'
dd-error-multiple-format-table = Seul un seul de conv=ascii conv=ebcdic ou conv=ibm peut être spécifié
dd-error-multiple-case = Seul un seul de conv=lcase ou conv=ucase peut être spécifié
dd-error-multiple-block = Seul un seul de conv=block ou conv=unblock peut être spécifié
dd-error-multiple-excl = Seul un seul de conv=excl ou conv=nocreat peut être spécifié
dd-error-invalid-flag = indicateur d'entrée invalide : '{ $flag }'
  Essayez '{ $cmd } --help' pour plus d'informations.
dd-error-conv-flag-no-match = conv=CONV non reconnu -> { $flag }
dd-error-multiplier-parse-failure = nombre invalide : ‘{ $input }‘
dd-error-multiplier-overflow = La chaîne de multiplicateur déborderait sur le système actuel -> { $input }
dd-error-block-without-cbs = conv=block ou conv=unblock spécifié sans cbs=N
dd-error-status-not-recognized = status=NIVEAU non reconnu -> { $level }
dd-error-unimplemented = fonctionnalité non implémentée sur ce système -> { $feature }
dd-error-bs-out-of-range = { $param }=N ne peut pas tenir en mémoire
dd-error-invalid-number = nombre invalide : ‘{ $input }‘

# Progress messages
dd-progress-records-in = { $complete }+{ $partial } enregistrements en entrée
dd-progress-records-out = { $complete }+{ $partial } enregistrements en sortie
dd-progress-truncated-record = { $count ->
    [one] { $count } enregistrement tronqué
   *[other] { $count } enregistrements tronqués
}
dd-progress-byte-copied = { $bytes } octet copié, { $duration } s, { $rate }/s
dd-progress-bytes-copied = { $bytes } octets copiés, { $duration } s, { $rate }/s
dd-progress-bytes-copied-si = { $bytes } octets ({ $si }) copiés, { $duration } s, { $rate }/s
dd-progress-bytes-copied-si-iec = { $bytes } octets ({ $si }, { $iec }) copiés, { $duration } s, { $rate }/s

# Warnings
dd-warning-zero-multiplier = { $zero } est un multiplicateur zéro ; utilisez { $alternative } si c'est voulu
dd-warning-signal-handler = Avertissement dd interne : Impossible d'enregistrer le gestionnaire de signal
