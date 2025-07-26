base32-about = encoder/décoder les données et les imprimer sur la sortie standard
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.

  Les données sont encodées comme décrit pour l'alphabet base32 dans RFC 4648.
  Lors du décodage, l'entrée peut contenir des retours à la ligne en plus
  des octets de l'alphabet base32 formel. Utilisez --ignore-garbage
  pour tenter de récupérer des autres octets non-alphabétiques dans
  le flux encodé.
base32-usage = base32 [OPTION]... [FICHIER]

base64-about = encoder/décoder les données et les imprimer sur la sortie standard
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.

  Les données sont encodées comme décrit pour l'alphabet base64 dans RFC 3548.
  Lors du décodage, l'entrée peut contenir des retours à la ligne en plus
  des octets de l'alphabet base64 formel. Utilisez --ignore-garbage
  pour tenter de récupérer des autres octets non-alphabétiques dans
  le flux encodé.
base64-usage = base64 [OPTION]... [FICHIER]

basenc-about = Encoder/décoder des données et afficher vers la sortie standard
  Sans FICHIER, ou lorsque FICHIER est -, lire l'entrée standard.

  Lors du décodage, l'entrée peut contenir des nouvelles lignes en plus des octets de
  l'alphabet formel. Utilisez --ignore-garbage pour tenter de récupérer
  depuis tout autre octet non-alphabétique dans le flux encodé.
basenc-usage = basenc [OPTION]... [FICHIER]

# Messages d'aide pour les formats d'encodage
basenc-help-base64 = identique au programme 'base64'
basenc-help-base64url = base64 sécurisé pour fichiers et URLs
basenc-help-base32 = identique au programme 'base32'
basenc-help-base32hex = base32 avec alphabet hexadécimal étendu
basenc-help-base16 = encodage hexadécimal
basenc-help-base2lsbf = chaîne de bits avec le bit de poids faible (lsb) en premier
basenc-help-base2msbf = chaîne de bits avec le bit de poids fort (msb) en premier
basenc-help-z85 = encodage de type ascii85 ;
  lors de l'encodage, la longueur d'entrée doit être un multiple de 4 ;
  lors du décodage, la longueur d'entrée doit être un multiple de 5

# Messages d'erreur
basenc-error-missing-encoding-type = type d'encodage manquant

# Messages d'erreur partagés de base_common (utilisés par base32, base64, basenc)
base-common-extra-operand = opérande supplémentaire {$operand}
base-common-no-such-file = {$file} : Aucun fichier ou répertoire de ce type
base-common-invalid-wrap-size = taille de retour à la ligne invalide : {$size}
base-common-read-error = erreur de lecture : {$error}

# Messages d'aide partagés de base_common
base-common-help-decode = décoder les données
base-common-help-ignore-garbage = lors du décodage, ignorer les caractères non-alphabétiques
base-common-help-wrap = retour à la ligne des lignes encodées après COLS caractères (par défaut {$default}, 0 pour désactiver le retour à la ligne)
