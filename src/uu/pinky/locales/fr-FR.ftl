pinky-about = Affiche des informations brèves sur les utilisateurs des systèmes Unix
pinky-usage = pinky [OPTION]... [UTILISATEUR]...
pinky-about-musl-warning = Avertissement : Lorsque compilé avec musl libc, l'utilitaire `pinky` peut afficher des
    informations utilisateur incomplètes ou manquantes en raison de l'implémentation
    factice des fonctions `utmpx` de musl. Cette limitation affecte la capacité
    à récupérer des détails précis sur les utilisateurs connectés.

# Description d'utilisation longue
pinky-long-usage-description = Un programme 'finger' léger ; affiche les informations utilisateur.
  Le fichier utmp sera

# Messages d'aide
pinky-help-long-format = produire une sortie au format long pour les UTILISATEURS spécifiés
pinky-help-omit-home-dir = omettre le répertoire personnel et le shell de l'utilisateur en format long
pinky-help-omit-project-file = omettre le fichier projet de l'utilisateur en format long
pinky-help-omit-plan-file = omettre le fichier plan de l'utilisateur en format long
pinky-help-short-format = faire une sortie au format court, c'est le défaut
pinky-help-omit-headings = omettre la ligne des en-têtes de colonnes en format court
pinky-help-omit-name = omettre le nom complet de l'utilisateur en format court
pinky-help-omit-name-host = omettre le nom complet et l'hôte distant de l'utilisateur en format court
pinky-help-omit-name-host-time = omettre le nom complet, l'hôte distant et le temps d'inactivité de l'utilisateur en format court
pinky-help-lookup = tenter de donner un forme canonique aux noms d'hôte avec DNS
pinky-help-help = Afficher les informations d'aide

# En-têtes de colonnes pour le format court
pinky-column-login = Connexion
pinky-column-name = Nom
pinky-column-tty =  TTY
pinky-column-idle = Inactif
pinky-column-when = Quand
pinky-column-where = Où

# Étiquettes pour le format long
pinky-login-name-label = Nom de connexion :
pinky-real-life-label = Dans la vraie vie :
pinky-directory-label = Répertoire :
pinky-shell-label = Shell :
pinky-project-label = Projet :
pinky-plan-label = Plan

# Messages de statut
pinky-unsupported-openbsd = commande non supportée sur OpenBSD
