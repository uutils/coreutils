uptime-about = Afficher l'heure actuelle, la durée pendant laquelle le système a été actif,
  le nombre d'utilisateurs sur le système, et le nombre moyen de tâches
  dans la file d'attente d'exécution au cours des 1, 5 et 15 dernières minutes.
uptime-usage = uptime [OPTION]...
uptime-about-musl-warning = Avertissement : Lorsque compilé avec musl libc, l'utilitaire `uptime` peut afficher '0 utilisateur'
    en raison de l'implémentation stub des fonctions utmpx de musl. L'heure de démarrage et les moyennes de charge
    sont toujours calculées en utilisant des mécanismes alternatifs.

# Messages d'aide
uptime-help-since = système actif depuis
uptime-help-path = fichier pour rechercher l'heure de démarrage

# Messages d'erreur
uptime-error-io = impossible d'obtenir l'heure de démarrage : { $error }
uptime-error-target-is-dir = impossible d'obtenir l'heure de démarrage : Est un répertoire
uptime-error-target-is-fifo = impossible d'obtenir l'heure de démarrage : Recherche illégale
uptime-error-couldnt-get-boot-time = impossible d'obtenir l'heure de démarrage

# Messages de sortie
uptime-output-unknown-uptime = actif ???? jours ??:??,

uptime-user-count = { $count ->
    [one] 1 utilisateur
   *[other] { $count } utilisateurs
}

# Messages d'erreur
uptime-lib-error-system-uptime = impossible de récupérer la durée de fonctionnement du système
uptime-lib-error-system-loadavg = impossible de récupérer la charge moyenne du système
uptime-lib-error-windows-loadavg = Windows n'a pas d'équivalent à la charge moyenne des systèmes de type Unix
uptime-lib-error-boot-time = heure de démarrage supérieure à l'heure actuelle

# Formatage de la durée de fonctionnement
uptime-format = { $days ->
    [0] { $time }
    [one] { $days } jour, { $time }
   *[other] { $days } jours { $time }
}

# Formatage de la charge moyenne
uptime-lib-format-loadavg = charge moyenne : { $avg1 }, { $avg5 }, { $avg15 }
