test-about = Vérifier les types de fichiers et comparer les valeurs.
test-usage = test EXPRESSION
  test
  {"[ EXPRESSION ]"}
  {"[ ]"}
  {"[ OPTION ]"}
test-after-help = Quitter avec le statut déterminé par EXPRESSION.

  Une EXPRESSION omise vaut false par défaut.
  Sinon, EXPRESSION est true ou false et définit le statut de sortie.

  Il peut s'agir de :

  - ( EXPRESSION ) EXPRESSION est vraie
  - ! EXPRESSION EXPRESSION est fausse
  - EXPRESSION1 -a EXPRESSION2 EXPRESSION1 et EXPRESSION2 sont toutes deux vraies
  - EXPRESSION1 -o EXPRESSION2 EXPRESSION1 ou EXPRESSION2 est vraie

  Opérations sur les chaînes :
  - -n STRING la longueur de STRING est non nulle
  - STRING équivalent à -n STRING
  - -z STRING la longueur de STRING est nulle
  - STRING1 = STRING2 les chaînes sont égales
  - STRING1 != STRING2 les chaînes ne sont pas égales

  Comparaisons d'entiers :
  - INTEGER1 -eq INTEGER2 INTEGER1 est égal à INTEGER2
  - INTEGER1 -ge INTEGER2 INTEGER1 est supérieur ou égal à INTEGER2
  - INTEGER1 -gt INTEGER2 INTEGER1 est supérieur à INTEGER2
  - INTEGER1 -le INTEGER2 INTEGER1 est inférieur ou égal à INTEGER2
  - INTEGER1 -lt INTEGER2 INTEGER1 est inférieur à INTEGER2
  - INTEGER1 -ne INTEGER2 INTEGER1 n'est pas égal à INTEGER2

  Opérations sur les fichiers :
  - FILE1 -ef FILE2 FILE1 et FILE2 ont les mêmes numéros de périphérique et d'inode
  - FILE1 -nt FILE2 FILE1 est plus récent (date de modification) que FILE2
  - FILE1 -ot FILE2 FILE1 est plus ancien que FILE2

  - -b FILE FILE existe et est un fichier spécial de type bloc
  - -c FILE FILE existe et est un fichier spécial de type caractère
  - -d FILE FILE existe et est un répertoire
  - -e FILE FILE existe
  - -f FILE FILE existe et est un fichier régulier
  - -g FILE FILE existe et a le bit set-group-ID
  - -G FILE FILE existe et appartient à l'ID de groupe effectif
  - -h FILE FILE existe et est un lien symbolique (identique à -L)
  - -k FILE FILE existe et a son bit sticky défini
  - -L FILE FILE existe et est un lien symbolique (identique à -h)
  - -N FILE FILE existe et a été modifié depuis sa dernière lecture
  - -O FILE FILE existe et appartient à l'ID utilisateur effectif
  - -p FILE FILE existe et est un tube nommé
  - -r FILE FILE existe et la permission de lecture est accordée
  - -s FILE FILE existe et a une taille supérieure à zéro
  - -S FILE FILE existe et est un socket
  - -t FD le descripteur de fichier FD est ouvert sur un terminal
  - -u FILE FILE existe et son bit set-user-ID est défini
  - -w FILE FILE existe et la permission d'écriture est accordée
  - -x FILE FILE existe et la permission d'exécution (ou de recherche) est accordée

  À l'exception de -h et -L, tous les tests liés aux FILE déréférencent (suivent) les liens symboliques.
  Attention : les parenthèses doivent être échappées (par exemple, par des barres obliques inverses) pour les shells.
  INTEGER peut aussi être -l STRING, qui évalue la longueur de STRING.

  NOTE : Les -a et -o binaires sont intrinsèquement ambigus.
  Utilisez test EXPR1 && test EXPR2 ou test EXPR1 || test EXPR2 à la place.
  NOTE : {"["} honore les options --help et --version, mais test ne le fait pas.
  test traite chacune de celles-ci comme il traite toute autre STRING non vide.
  NOTE : votre shell peut avoir sa propre version de test et/ou {"["}, qui remplace généralement la version décrite ici.
  Veuillez vous référer à la documentation de votre shell pour les détails sur les options qu'il prend en charge.

# Messages d'erreur
test-error-missing-closing-bracket = '{"]"}' manquant
test-error-expected = { $value } attendu
test-error-expected-value = valeur attendue
test-error-missing-argument = argument manquant après { $argument }
test-error-extra-argument = argument supplémentaire { $argument }
test-error-unknown-operator = opérateur inconnu { $operator }
test-error-invalid-integer = entier invalide { $value }
test-error-unary-operator-expected = { $operator } : opérateur unaire attendu
