printf-about = Afficher la sortie basée sur la chaîne de format et les arguments suivants.
printf-usage = printf FORMAT [ARGUMENT]...
  printf OPTION
printf-after-help = templating de chaîne anonyme de base :

  affiche la chaîne de format au moins une fois, se répétant tant qu'il reste des arguments
  la sortie affiche les littéraux échappés dans la chaîne de format comme des littéraux de caractères
  la sortie remplace les champs anonymes par le prochain argument inutilisé, formaté selon le champ.

  Affiche le , en remplaçant les séquences de caractères échappés par des littéraux de caractères
  et les séquences de champs de substitution par les arguments passés

  littéralement, à l'exception de ce qui suit
  séquences de caractères échappés, et les séquences de substitution décrites plus loin.

  ### SÉQUENCES D'ÉCHAPPEMENT

  Les séquences d'échappement suivantes, organisées ici par ordre alphabétique,
  afficheront le littéral de caractère correspondant :

  - \" guillemet double

  - \\ barre oblique inverse

  - \\a alerte (BEL)

  - \\b retour arrière

  - \\c Fin d'entrée

  - \\e échappement

  - \\f saut de page

  - \\n nouvelle ligne

  - \\r retour chariot

  - \\t tabulation horizontale

  - \\v tabulation verticale

  - \\NNN octet avec valeur exprimée en valeur octale NNN (1 à 3 chiffres)
            les valeurs supérieures à 256 seront traitées

  - \\xHH octet avec valeur exprimée en valeur hexadécimale NN (1 à 2 chiffres)

  - \\uHHHH caractère Unicode (IEC 10646) avec valeur exprimée en valeur hexadécimale HHHH (4 chiffres)

  - \\uHHHH caractère Unicode avec valeur exprimée en valeur hexadécimale HHHH (8 chiffres)

  - %% un seul %

  ### SUBSTITUTIONS

  #### RÉFÉRENCE RAPIDE DES SUBSTITUTIONS

  Champs

  - %s: chaîne
  - %b: chaîne analysée pour les littéraux, le deuxième paramètre est la longueur max

  - %c: caractère, pas de deuxième paramètre

  - %i ou %d: entier 64 bits
  - %u: entier non signé 64 bits
  - %x ou %X: entier non signé 64 bits en hexadécimal
  - %o: entier non signé 64 bits en octal
              le deuxième paramètre est la largeur min, entier
              la sortie en dessous de cette largeur est remplie avec des zéros en tête

  - %q: ARGUMENT est affiché dans un format qui peut être réutilisé comme entrée shell, en échappant les
              caractères non imprimables avec la syntaxe POSIX $'' proposée.

  - %f ou %F: valeur en virgule flottante décimale
  - %e ou %E: valeur en virgule flottante en notation scientifique
  - %g ou %G: plus courte des valeurs en virgule flottante décimale ou SciNote interprétées spécialement.
              le deuxième paramètre est
                -max places après la virgule pour la sortie en virgule flottante
                -max nombre de chiffres significatifs pour la sortie en notation scientifique

  paramétrage des champs

  exemples :

  printf '%4.3i' 7

  Il a un premier paramètre de 4 et un deuxième paramètre de 3 et donnera ' 007'

  printf '%.1s' abcde

  Il n'a pas de premier paramètre et un deuxième paramètre de 1 et donnera 'a'

  printf '%4c' q

  Il a un premier paramètre de 4 et pas de deuxième paramètre et donnera ' q'

  Le premier paramètre d'un champ est la largeur minimale pour remplir la sortie
  si la sortie est inférieure à cette valeur absolue de cette largeur,
  elle sera remplie avec des espaces en tête, ou, si l'argument est négatif,
  avec des espaces en queue. la valeur par défaut est zéro.

  Le deuxième paramètre d'un champ est particulier au type de champ de sortie.
  les valeurs par défaut peuvent être trouvées dans l'aide de substitution complète ci-dessous

  préfixes spéciaux pour les arguments numériques

  - 0: (ex. 010) interpréter l'argument comme octal (champs de sortie entiers uniquement)
  - 0x: (ex. 0xABC) interpréter l'argument comme hexadécimal (champs de sortie numériques uniquement)
  - \': (ex. \'a) interpréter l'argument comme une constante de caractère

  #### COMMENT UTILISER LES SUBSTITUTIONS

  Les substitutions sont utilisées pour passer des argument(s) supplémentaire(s) dans la chaîne FORMAT, pour être formatés d'une
  manière particulière. Par ex.

  printf 'la lettre %X vient avant la lettre %X' 10 11

  affichera

  la lettre A vient avant la lettre B

  parce que le champ de substitution %X signifie
  'prendre un argument entier et l'écrire comme un nombre hexadécimal'

  Passer plus d'arguments qu'il n'y en a dans la chaîne de format fera que la chaîne de format sera
  répétée pour les substitutions restantes

  printf 'il fait %i F à %s \n' 22 Portland 25 Boston 27 New York

  affichera

  il fait 22 F à Portland
  il fait 25 F à Boston
  il fait 27 F à Boston

  Si une chaîne de format est affichée mais qu'il reste moins d'arguments
  qu'il n'y a de champs de substitution, les champs de substitution sans
  argument auront par défaut des chaînes vides, ou pour les champs numériques
  la valeur 0

  #### SUBSTITUTIONS DISPONIBLES

  Ce programme, comme GNU coreutils printf,
  interprète un sous-ensemble modifié de la spécification printf C POSIX,
  une référence rapide aux substitutions est ci-dessous.

  #### SUBSTITUTIONS DE CHAÎNES

  Tous les champs de chaîne ont un paramètre 'largeur max'
  %.3s signifie 'afficher pas plus de trois caractères de l'entrée originale'

  - %s: chaîne

  - %b: chaîne échappée - la chaîne sera vérifiée pour tout littéral échappé de
        la liste de littéraux échappés ci-dessus, et les traduire en caractères littéraux.
        ex. \\n sera transformé en caractère de nouvelle ligne.
        Une règle spéciale sur le mode %b est que les littéraux octaux sont interprétés différemment
        Dans les arguments passés par %b, les littéraux interprétés en octal doivent être sous la forme \\0NNN
        au lieu de \\NNN. (Bien que, pour des raisons d'héritage, les littéraux octaux sous la forme \\NNN seront
        toujours interprétés et ne lanceront pas d'avertissement, vous aurez des problèmes si vous utilisez cela pour un
        littéral dont le code commence par zéro, car il sera vu comme étant sous forme \\0NNN.)

  - %q: chaîne échappée - la chaîne dans un format qui peut être réutilisé comme entrée par la plupart des shells.
        Les caractères non imprimables sont échappés avec la syntaxe POSIX proposée '$''',
        et les méta-caractères shell sont cités de manière appropriée.
        C'est un format équivalent à la sortie ls --quoting=shell-escape.

  #### SUBSTITUTIONS DE CARACTÈRES

  Le champ caractère n'a pas de paramètre secondaire.

  - %c: un seul caractère

  #### SUBSTITUTIONS D'ENTIERS

  Tous les champs entiers ont un paramètre 'remplir avec zéro'
  %.4i signifie un entier qui s'il fait moins de 4 chiffres de longueur,
  est rempli avec des zéros en tête jusqu'à ce qu'il fasse 4 chiffres de longueur.

  - %d ou %i: entier 64 bits

  - %u: entier non signé 64 bits

  - %x ou %X: entier non signé 64 bits affiché en hexadécimal (base 16)
              %X au lieu de %x signifie utiliser des lettres majuscules pour 'a' à 'f'

  - %o: entier non signé 64 bits affiché en octal (base 8)

  #### SUBSTITUTIONS EN VIRGULE FLOTTANTE

  Tous les champs en virgule flottante ont un paramètre 'max places décimales / max chiffres significatifs'
  %.10f signifie une virgule flottante décimale avec 7 places décimales après 0
  %.10e signifie un nombre en notation scientifique avec 10 chiffres significatifs
  %.10g signifie le même comportement pour décimal et Sci. Note, respectivement, et fournit la plus courte
  de chaque sortie.

  Comme avec GNU coreutils, la valeur après la virgule de ces sorties est analysée comme un
  double d'abord avant d'être rendue en texte. Pour les deux implémentations, n'attendez pas de précision significative
  au-delà de la 18ème place décimale. Lors de l'utilisation d'un nombre de places décimales qui est 18 ou
  plus élevé, vous pouvez vous attendre à une variation de sortie entre GNU coreutils printf et ce printf à la
  18ème place décimale de +/- 1

  - %f: valeur en virgule flottante présentée en décimal, tronquée et affichée à 6 places décimales par
        défaut. Il n'y a pas de parité de comportement post-double avec Coreutils printf, les valeurs ne sont pas
        estimées ou ajustées au-delà des valeurs d'entrée.

  - %e ou %E: valeur en virgule flottante présentée en notation scientifique
              7 chiffres significatifs par défaut
              %E signifie utiliser E majuscule pour la mantisse.

  - %g ou %G: valeur en virgule flottante présentée dans la plus courte des notations décimale et scientifique
              se comporte différemment de %f et %E, veuillez voir la spécification posix printf pour tous les détails,
              quelques exemples de comportement différent :
              Sci Note a 6 chiffres significatifs par défaut
              Les zéros de fin sont supprimés
              Au lieu d'être tronqué, le chiffre après le dernier est arrondi

  Comme d'autres comportements dans cet utilitaire, les choix de conception du comportement en virgule flottante
  dans cet utilitaire sont sélectionnés pour reproduire exactement
  le comportement de printf de GNU coreutils du point de vue des entrées et sorties.

  ### UTILISATION DES PARAMÈTRES

  La plupart des champs de substitution peuvent être paramétrés en utilisant jusqu'à 2 nombres qui peuvent
  être passés au champ, entre le signe % et la lettre du champ.

  Le 1er paramètre indique toujours la largeur minimale de sortie, il est utile pour créer
  une sortie en colonnes. Toute sortie qui serait inférieure à cette largeur minimale est remplie avec
  des espaces en tête
  Le 2ème paramètre est précédé d'un point.
  Vous n'êtes pas obligé d'utiliser des paramètres

  ### FORMES SPÉCIALES D'ENTRÉE

  Pour l'entrée numérique, les formes d'entrée supplémentaires suivantes sont acceptées en plus du décimal :

  Octal (uniquement avec entier) : si l'argument commence par un 0, les caractères suivants
  seront interprétés comme octal (base 8) pour les champs entiers

  Hexadécimal : si l'argument commence par 0x, les caractères suivants seront interprétés
  seront interprétés comme hexadécimal (base 16) pour tous les champs numériques
  pour les champs flottants, l'entrée hexadécimale résulte en une limite de précision
  (dans la conversion de l'entrée après la virgule) de 10^-15

  Constante de caractère : si l'argument commence par un caractère guillemet simple, le premier octet
  du caractère suivant sera interprété comme un entier non signé 8 bits. S'il y a
  des octets supplémentaires, ils lanceront une erreur (sauf si la variable d'environnement POSIXLY_CORRECT
  est définie)

# Messages d'erreur
printf-error-missing-operand = opérande manquant
printf-warning-ignoring-excess-arguments = arguments excédentaires ignorés, en commençant par { $arg }
printf-help-version = Afficher les informations de version
printf-help-help = Afficher cette aide
