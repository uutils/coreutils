dircolors-about = Afficher les commandes pour définir la variable d'environnement LS_COLORS.
dircolors-usage = dircolors [OPTION]... [FICHIER]
dircolors-after-help = Si FICHIER est spécifié, le lire pour déterminer quelles couleurs utiliser pour quels
  types de fichiers et extensions. Sinon, une base de données précompilée est utilisée.
  Pour les détails sur le format de ces fichiers, exécutez 'dircolors --print-database'

# Messages d'aide
dircolors-help-bourne-shell = afficher le code Bourne shell pour définir LS_COLORS
dircolors-help-c-shell = afficher le code C shell pour définir LS_COLORS
dircolors-help-print-database = afficher la base de données de configuration
dircolors-help-print-ls-colors = afficher les couleurs entièrement échappées pour l'affichage

# Messages d'erreur
dircolors-error-shell-and-output-exclusive = les options pour afficher une syntaxe non-shell
  et pour sélectionner une syntaxe shell sont mutuellement exclusives
dircolors-error-print-database-and-ls-colors-exclusive = les options --print-database et --print-ls-colors sont mutuellement exclusives
dircolors-error-extra-operand-print-database = opérande supplémentaire { $operand }
  les opérandes de fichier ne peuvent pas être combinées avec --print-database (-p)
dircolors-error-no-shell-environment = aucune variable d'environnement SHELL, et aucune option de type de shell donnée
dircolors-error-extra-operand = opérande supplémentaire { $operand }
dircolors-error-expected-file-got-directory = fichier attendu, répertoire obtenu { $path }
dircolors-error-invalid-line-missing-token = { $file }:{ $line } : ligne invalide ; jeton manquant
dircolors-error-unrecognized-keyword = mot-clé non reconnu { $keyword }
