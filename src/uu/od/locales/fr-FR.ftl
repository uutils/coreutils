od-about = Afficher les fichiers en format octal et autres formats
od-usage = od [OPTION]... [--] [NOMFICHIER]...
  od [-abcdDefFhHiIlLoOsxX] [NOMFICHIER] [[+][0x]DÉCALAGE[.][b]]
  od --traditional [OPTION]... [NOMFICHIER] [[+][0x]DÉCALAGE[.][b] [[+][0x]ÉTIQUETTE[.][b]]]
od-after-help = Affiche les données dans divers formats lisibles par l'homme. Si plusieurs
  formats sont spécifiés, la sortie contiendra tous les formats dans l'ordre où ils
  apparaissent sur la ligne de commande. Chaque format sera imprimé sur une nouvelle
  ligne. Seule la ligne contenant le premier format sera préfixée avec le décalage.

  Si aucun nom de fichier n'est spécifié, ou si c'est "-", stdin sera utilisé. Après
  un "--", aucune autre option ne sera reconnue. Cela permet d'avoir des noms de
  fichiers commençant par "-".

  Si un nom de fichier est un nombre valide qui peut être utilisé comme décalage dans
  la deuxième forme, vous pouvez le forcer à être reconnu comme un nom de fichier si
  vous incluez une option comme "-j0", qui n'est valide que dans la première forme.

  RADIX est l'un de o,d,x,n pour octal, décimal, hexadécimal ou aucun.

  OCTETS est décimal par défaut, octal si préfixé par "0", ou hexadécimal si préfixé
  par "0x". Les suffixes b, KB, K, MB, M, GB, G multiplieront le nombre par 512,
  1000, 1024, 1000^2, 1024^2, 1000^3, 1024^3, 1000^2, 1024^2.

  DÉCALAGE et ÉTIQUETTE sont octaux par défaut, hexadécimaux si préfixés par "0x" ou
  décimaux si un suffixe "." est ajouté. Le suffixe "b" multipliera par 512.

  TYPE contient une ou plusieurs spécifications de format constituées de :
      a pour ASCII imprimable 7-bits
      c pour caractères utf-8 ou octal pour caractères non définis
      d[TAILLE] pour décimal signé
      f[TAILLE] pour virgule flottante
      o[TAILLE] pour octal
      u[TAILLE] pour décimal non signé
      x[TAILLE] pour hexadécimal
  TAILLE est le nombre d'octets qui peut être le nombre 1, 2, 4, 8 ou 16,
      ou C, I, S, L pour 1, 2, 4, 8 octets pour les types entiers,
      ou F, D, L pour 4, 8, 16 octets pour les virgules flottantes.
  Toute spécification de type peut avoir un suffixe "z", qui ajoutera un dump ASCII
      à la fin de la ligne.

  Si une erreur s'est produite, un message de diagnostic sera imprimé sur stderr, et
  le code de sortie sera non-zéro.

# Messages d'erreur
od-error-invalid-endian = Argument invalide --endian={$endian}
od-error-invalid-inputs = Entrées invalides : {$msg}
od-error-too-large = la valeur est trop grande
od-error-radix-invalid = Radix doit être l'un de [o, d, x, n], reçu : {$radix}
od-error-radix-empty = Radix ne peut pas être vide, et doit être l'un de [o, d, x, n]
od-error-invalid-width = largeur invalide {$width} ; utilisation de {$min} à la place
od-error-missing-format-spec = spécification de format manquante après '--format' / '-t'
od-error-unexpected-char = caractère inattendu '{$char}' dans la spécification de format {$spec}
od-error-invalid-number = nombre invalide {$number} dans la spécification de format {$spec}
od-error-invalid-size = taille invalide '{$size}' dans la spécification de format {$spec}
od-error-invalid-offset = décalage invalide : {$offset}
od-error-invalid-label = étiquette invalide : {$label}
od-error-too-many-inputs = trop d'entrées après --traditional : {$input}
od-error-parse-failed = échec de l'analyse
od-error-invalid-suffix = suffixe invalide dans l'argument --{$option} {$value}
od-error-invalid-argument = argument --{$option} invalide {$value}
od-error-argument-too-large = argument --{$option} {$value} trop grand
od-error-skip-past-end = tentative d'ignorer au-delà de la fin de l'entrée

# Messages d'aide
od-help-help = Afficher les informations d'aide.
od-help-address-radix = Sélectionner la base dans laquelle les décalages de fichier sont imprimés.
od-help-skip-bytes = Ignorer les octets d'entrée avant le formatage et l'écriture.
od-help-read-bytes = limiter le dump à OCTETS octets d'entrée
od-help-endian = ordre des octets à utiliser pour les formats multi-octets
od-help-a = caractères nommés, ignorant le bit d'ordre supérieur
od-help-b = octets octaux
od-help-c = caractères ASCII ou échappements antislash
od-help-d = unités décimales non signées 2-octets
od-help-d4 = unités décimales non signées 4-octets
od-help-format = sélectionner le(s) format(s) de sortie
od-help-output-duplicates = ne pas utiliser * pour marquer la suppression de ligne
od-help-width = sortir OCTETS octets par ligne de sortie. 32 est impliqué quand OCTETS n'est pas
                spécifié.
od-help-traditional = mode de compatibilité avec une entrée, décalage et étiquette.
