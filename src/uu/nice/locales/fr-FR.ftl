nice-about = Exécute COMMANDE avec une priorité ajustée, ce qui affecte l'ordonnancement des processus.
  Sans COMMANDE, affiche la priorité actuelle. Les valeurs de priorité vont de
  -20 (plus favorable au processus) à 19 (moins favorable au processus).
nice-usage = nice [OPTION] [COMMANDE [ARG]...]

# Messages d'erreur
nice-error-command-required-with-adjustment = Une commande doit être fournie avec un ajustement.
nice-error-invalid-number = "{ $value }" n'est pas un nombre valide : { $error }
nice-warning-setpriority = { $util_name } : avertissement : setpriority : { $error }

# Texte d'aide pour les arguments de ligne de commande
nice-help-adjustment = ajoute N à la priorité (par défaut 10)
