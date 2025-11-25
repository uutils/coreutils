cksum-about = Afficher le CRC et la taille de chaque fichier
cksum-usage = cksum [OPTION]... [FICHIER]...
cksum-after-help = DIGEST détermine l'algorithme de condensé et le format de sortie par défaut :

  - sysv : (équivalent à sum -s)
  - bsd : (équivalent à sum -r)
  - crc : (équivalent à cksum)
  - crc32b : (disponible uniquement via cksum)
  - md5 : (équivalent à md5sum)
  - sha1 : (équivalent à sha1sum)
  - sha2: (équivalent à sha{"{224,256,384,512}"}sum)
  - sha3 : (disponible uniquement via cksum)
  - blake2b : (équivalent à b2sum)
  - sm3 : (disponible uniquement via cksum)

# Messages d'aide
cksum-help-algorithm = sélectionner le type de condensé à utiliser. Voir DIGEST ci-dessous
cksum-help-untagged = créer une somme de contrôle de style inversé, sans type de condensé
cksum-help-tag = créer une somme de contrôle de style BSD, annuler --untagged (par défaut)
cksum-help-length = longueur du condensé en bits ; ne doit pas dépasser le maximum pour l'algorithme blake2 et doit être un multiple de 8
cksum-help-raw = émettre un condensé binaire brut, pas hexadécimal
cksum-help-strict = sortir avec un code non-zéro pour les lignes de somme de contrôle mal formatées
cksum-help-check = lire les sommes de hachage des FICHIERs et les vérifier
cksum-help-base64 = émettre un condensé base64, pas hexadécimal
cksum-help-warn = avertir des lignes de somme de contrôle mal formatées
cksum-help-status = ne rien afficher, le code de statut indique le succès
cksum-help-quiet = ne pas afficher OK pour chaque fichier vérifié avec succès
cksum-help-ignore-missing = ne pas échouer ou signaler le statut pour les fichiers manquants
cksum-help-zero = terminer chaque ligne de sortie avec NUL, pas un saut de ligne, et désactiver l'échappement des noms de fichiers

# Messages d'erreur
cksum-error-is-directory = { $file } : Est un répertoire
cksum-error-failed-to-read-input = échec de la lecture de l'entrée
