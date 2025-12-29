env-about = Définir chaque NOM à VALEUR dans l'environnement et exécuter COMMANDE
env-usage = env [OPTION]... [-] [NOM=VALEUR]... [COMMANDE [ARG]...]
env-after-help = Un simple - implique -i. Si aucune COMMANDE, afficher l'environnement résultant.

# Messages d'aide
env-help-ignore-environment = commencer avec un environnement vide
env-help-chdir = changer le répertoire de travail vers RÉP
env-help-null = terminer chaque ligne de sortie avec un octet 0 plutôt qu'un retour à la ligne (valide uniquement lors de l'affichage de l'environnement)
env-help-file = lire et définir les variables à partir d'un fichier de configuration de style ".env" (avant toute suppression et/ou définition)
env-help-unset = supprimer la variable de l'environnement
env-help-debug = afficher des informations détaillées pour chaque étape de traitement
env-help-split-string = traiter et diviser S en arguments séparés ; utilisé pour passer plusieurs arguments sur les lignes shebang
env-help-argv0 = Remplacer le zéroième argument passé à la commande en cours d'exécution. Sans cette option, une valeur par défaut de `command` est utilisée.
env-help-ignore-signal = définir la gestion du/des signal/signaux SIG pour ne rien faire
env-help-default-signal = réinitialiser la gestion du/des signal/signaux SIG à l'action par défaut
env-help-block-signal = bloquer la livraison du/des signal/signaux SIG pendant l'exécution de COMMAND
env-help-list-signal-handling = lister les traitements de signaux modifiés par les options précédentes

# Messages d'erreur
env-error-missing-closing-quote = aucune guillemet de fermeture dans la chaîne -S à la position { $position } pour la guillemet '{ $quote }'
env-error-invalid-backslash-at-end = barre oblique inverse invalide à la fin de la chaîne dans -S à la position { $position } dans le contexte { $context }
env-error-backslash-c-not-allowed = '\\c' ne doit pas apparaître dans une chaîne -S entre guillemets doubles à la position { $position }
env-error-invalid-sequence = séquence invalide '\\{ $char }' dans -S à la position { $position }
env-error-missing-closing-brace = Accolade fermante manquante à la position { $position }
env-error-missing-variable = Nom de variable manquant à la position { $position }
env-error-missing-closing-brace-after-value = Accolade fermante manquante après la valeur par défaut à la position { $position }
env-error-unexpected-number = Caractère inattendu : '{ $char }', le nom de variable attendu ne doit pas commencer par 0..9 à la position { $position }
env-error-expected-brace-or-colon = Caractère inattendu : '{ $char }', accolade fermante ('{"\\}"}') ou deux-points (':') attendu à la position { $position }
env-error-cannot-specify-null-with-command = impossible de spécifier --null (-0) avec une commande
env-error-invalid-signal = { $signal } : signal invalide

env-error-config-file = { $file } : { $error }
env-error-variable-name-issue = problème de nom de variable (à { $position }) : { $error }
env-error-generic = Erreur : { $error }
env-error-no-such-file = { $program } : Aucun fichier ou répertoire de ce type
env-error-use-s-shebang = utilisez -[v]S pour passer des options dans les lignes shebang
env-error-cannot-unset = impossible de supprimer '{ $name }' : Argument invalide
env-error-cannot-unset-invalid = impossible de supprimer { $name } : Argument invalide
env-error-must-specify-command-with-chdir = doit spécifier une commande avec --chdir (-C)
env-error-cannot-change-directory = impossible de changer de répertoire vers { $directory } : { $error }
env-error-argv0-not-supported = --argv0 n'est actuellement pas supporté sur cette plateforme
env-error-permission-denied = { $program } : Permission refusée
env-error-unknown = erreur inconnue : { $error }
env-error-failed-set-signal-action = échec de la définition de l'action du signal pour le signal { $signal } : { $error }

# Messages d'avertissement
env-warning-no-name-specified = aucun nom spécifié pour la valeur { $value }
