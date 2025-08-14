runcon-about = Exécuter une commande avec le contexte de sécurité spécifié sous les systèmes avec SELinux activé.
runcon-usage = runcon CONTEXTE COMMANDE [ARG...]
  runcon [-c] [-u UTILISATEUR] [-r RÔLE] [-t TYPE] [-l PLAGE] COMMANDE [ARG...]
runcon-after-help = Exécuter COMMANDE avec un CONTEXTE complètement spécifié, ou avec le contexte de sécurité actuel ou de transition modifié par un ou plusieurs parmi NIVEAU, RÔLE, TYPE et UTILISATEUR.

  Si aucun de --compute, --type, --user, --role ou --range n'est spécifié, alors le premier argument est utilisé comme contexte complet.

  Notez que seuls les contextes soigneusement choisis ont des chances de s'exécuter avec succès.

  Si ni CONTEXTE ni COMMANDE n'est spécifié, le contexte de sécurité actuel est affiché.

# Messages d'aide
runcon-help-compute = Calculer le contexte de transition de processus avant modification.
runcon-help-user = Définir l'utilisateur UTILISATEUR dans le contexte de sécurité cible.
runcon-help-role = Définir le rôle RÔLE dans le contexte de sécurité cible.
runcon-help-type = Définir le type TYPE dans le contexte de sécurité cible.
runcon-help-range = Définir la plage PLAGE dans le contexte de sécurité cible.

# Messages d'erreur
runcon-error-no-command = Aucune commande n'est spécifiée
runcon-error-selinux-not-enabled = runcon ne peut être utilisé que sur un noyau SELinux
runcon-error-operation-failed = { $operation } a échoué
runcon-error-operation-failed-on = { $operation } a échoué sur { $operand }

# Noms d'opération
runcon-operation-getting-current-context = Obtention du contexte de sécurité du processus actuel
runcon-operation-creating-context = Création d'un nouveau contexte
runcon-operation-checking-context = Vérification du contexte de sécurité
runcon-operation-setting-context = Définition du nouveau contexte de sécurité
runcon-operation-getting-process-class = Obtention de la classe de sécurité du processus
runcon-operation-getting-file-context = Obtention du contexte de sécurité du fichier de commande
runcon-operation-computing-transition = Calcul du résultat de la transition de processus
runcon-operation-getting-context = Obtention du contexte de sécurité
runcon-operation-setting-user = Définition de l'utilisateur du contexte de sécurité
runcon-operation-setting-role = Définition du rôle du contexte de sécurité
runcon-operation-setting-type = Définition du type du contexte de sécurité
runcon-operation-setting-range = Définition de la plage du contexte de sécurité
runcon-operation-executing-command = Exécution de la commande
