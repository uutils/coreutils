expr-about = Afficher la valeur de EXPRESSION sur la sortie standard
expr-usage = expr [EXPRESSION]
  expr [OPTIONS]
expr-after-help = Afficher la valeur de EXPRESSION sur la sortie standard. Une ligne vide ci-dessous
  sépare les groupes de précédence croissante.

  EXPRESSION peut être :

  - ARG1 | ARG2: ARG1 s'il n'est ni nul ni 0, sinon ARG2
  - ARG1 & ARG2: ARG1 si aucun argument n'est nul ou 0, sinon 0
  - ARG1 < ARG2: ARG1 est inférieur à ARG2
  - ARG1 <= ARG2: ARG1 est inférieur ou égal à ARG2
  - ARG1 = ARG2: ARG1 est égal à ARG2
  - ARG1 != ARG2: ARG1 est différent de ARG2
  - ARG1 >= ARG2: ARG1 est supérieur ou égal à ARG2
  - ARG1 > ARG2: ARG1 est supérieur à ARG2
  - ARG1 + ARG2: somme arithmétique de ARG1 et ARG2
  - ARG1 - ARG2: différence arithmétique de ARG1 et ARG2
  - ARG1 * ARG2: produit arithmétique de ARG1 et ARG2
  - ARG1 / ARG2: quotient arithmétique de ARG1 divisé par ARG2
  - ARG1 % ARG2: reste arithmétique de ARG1 divisé par ARG2
  - STRING : REGEXP: correspondance de motif ancré de REGEXP dans STRING
  - match STRING REGEXP: identique à STRING : REGEXP
  - substr STRING POS LENGTH: sous-chaîne de STRING, POS compté à partir de 1
  - index STRING CHARS: index dans STRING où l'un des CHARS est trouvé, ou 0
  - length STRING: longueur de STRING
  - + TOKEN: interpréter TOKEN comme une chaîne, même si c'est un mot-clé comme match
    ou un opérateur comme /
  - ( EXPRESSION ): valeur de EXPRESSION

  Attention : de nombreux opérateurs doivent être échappés ou mis entre guillemets pour les shells.
  Les comparaisons sont arithmétiques si les deux ARG sont des nombres, sinon lexicographiques.
  Les correspondances de motifs retournent la chaîne correspondant entre \( et \) ou null ; si
  \( et \) ne sont pas utilisés, elles retournent le nombre de caractères correspondants ou 0.

  Le statut de sortie est 0 si EXPRESSION n'est ni nulle ni 0, 1 si EXPRESSION
  est nulle ou 0, 2 si EXPRESSION est syntaxiquement invalide, et 3 si une
  erreur s'est produite.

  Variables d'environnement :

  - EXPR_DEBUG_TOKENS=1: afficher les jetons de l'expression
  - EXPR_DEBUG_RPN=1: afficher l'expression représentée en notation polonaise inverse
  - EXPR_DEBUG_SYA_STEP=1: afficher chaque étape de l'analyseur
  - EXPR_DEBUG_AST=1: afficher l'arbre de syntaxe abstraite représentant l'expression

# Messages d'aide
expr-help-version = afficher les informations de version et quitter
expr-help-help = afficher cette aide et quitter

# Messages d'erreur
expr-error-unexpected-argument = erreur de syntaxe : argument inattendu { $arg }
expr-error-missing-argument = erreur de syntaxe : argument manquant après { $arg }
expr-error-non-integer-argument = argument non entier
expr-error-missing-operand = opérande manquant
expr-error-division-by-zero = division par zéro
expr-error-invalid-regex-expression = Expression regex invalide
expr-error-expected-closing-brace-after = erreur de syntaxe : ')' attendu après { $arg }
expr-error-expected-closing-brace-instead-of = erreur de syntaxe : ')' attendu au lieu de { $arg }
expr-error-unmatched-opening-parenthesis = Parenthèse ouvrante ( ou \( non appariée
expr-error-unmatched-closing-parenthesis = Parenthèse fermante ) ou \) non appariée
expr-error-unmatched-opening-brace = Accolade ouvrante {"\\{"} non appariée
expr-error-invalid-bracket-content = Contenu invalide de {"\\{\\}"}
expr-error-trailing-backslash = Barre oblique inverse en fin
expr-error-too-big-range-quantifier-index = Expression régulière trop grande
expr-error-match-utf8 = match ne supporte pas l'encodage UTF-8 invalide dans { $arg }
