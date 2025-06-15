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
