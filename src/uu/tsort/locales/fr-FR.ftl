tsort-about = Tri topologique des chaînes présentes dans FILE.
  Les chaînes sont définies comme toute séquence de jetons séparés par des espaces (tabulation, espace ou saut de ligne), ordonnées selon les dépendances dans un graphe orienté acyclique (DAG).
  Utile pour la planification et la détermination de l'ordre d'exécution.
  Si FILE n'est pas fourni, l'entrée standard (stdin) est utilisée.
tsort-usage = tsort [OPTIONS] FILE
tsort-error-is-dir = erreur de lecture : c'est un répertoire
tsort-error-odd = l'entrée contient un nombre impair de jetons
tsort-error-loop = l'entrée contient une boucle :
tsort-error-extra-operand = opérande supplémentaire { $operand }
  Essayez '{ $util } --help' pour plus d'informations.
tsort-error-at-least-one-input = au moins une entrée
