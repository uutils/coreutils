who-about = Affiche des informations sur les utilisateurs actuellement connectés.
who-usage = who [OPTION]... [ FICHIER | ARG1 ARG2 ]
who-about-musl-warning = Note : Lors de la compilation avec musl libc, l'utilitaire `who` n'affichera aucune
    information sur les utilisateurs connectés. Ceci est dû à l'implémentation
    stub des fonctions `utmpx` de musl, qui empêche l'accès aux données nécessaires.

who-long-usage = Si FICHIER n'est pas spécifié, utilise { $default_file }. /var/log/wtmp comme FICHIER est courant.
    Si ARG1 ARG2 sont donnés, -m est présumé : 'am i' ou 'mom likes' sont usuels.

# Help text for command-line arguments
who-help-all = identique à -b -d --login -p -r -t -T -u
who-help-boot = heure du dernier démarrage système
who-help-dead = affiche les processus morts
who-help-heading = affiche une ligne d'en-têtes de colonnes
who-help-login = affiche les processus de connexion système
who-help-lookup = tente de canonicaliser les noms d'hôtes via DNS
who-help-only-hostname-user = seulement le nom d'hôte et l'utilisateur associés à stdin
who-help-process = affiche les processus actifs lancés par init
who-help-count = tous les noms de connexion et le nombre d'utilisateurs connectés
who-help-runlevel = affiche le niveau d'exécution actuel
who-help-runlevel-non-linux = affiche le niveau d'exécution actuel (Sans signification sur non Linux)
who-help-short = affiche seulement nom, ligne et heure (par défaut)
who-help-time = affiche le dernier changement d'horloge système
who-help-users = liste les utilisateurs connectés
who-help-mesg = ajoute le statut de message de l'utilisateur comme +, - ou ?

# Output messages
who-user-count = # utilisateurs={ $count }

# Idle time indicators
who-idle-old = anc.
who-idle-unknown =   ?

# System information
who-runlevel = niveau-exec { $level }
who-runlevel-last = dernier={ $last }
who-clock-change = changement horloge
who-login = CONNEXION
who-login-id = id={ $id }
who-dead-exit-status = term={ $term } sortie={ $exit }
who-system-boot = démarrage système

# Table headings
who-heading-name = NOM
who-heading-line = LIGNE
who-heading-time = HEURE
who-heading-idle = INACTIF
who-heading-pid = PID
who-heading-comment = COMMENTAIRE
who-heading-exit = SORTIE

# Error messages
who-canonicalize-error = échec de canonicalisation de { $host }

# Platform-specific messages
who-unsupported-openbsd = commande non supportée sur OpenBSD
