timeout-about = Démarrer COMMANDE, et la tuer si elle fonctionne encore après DURÉE.
timeout-usage = timeout [OPTION] DURÉE COMMANDE...

# Messages d'aide
timeout-help-foreground = quand on n'exécute pas timeout directement depuis une invite de shell, permettre à COMMANDE de lire depuis le TTY et d'obtenir les signaux TTY ; dans ce mode, les enfants de COMMANDE ne seront pas limités dans le temps
timeout-help-kill-after = envoyer aussi un signal KILL si COMMANDE fonctionne encore si longtemps après que le signal initial ait été envoyé
timeout-help-preserve-status = sortir avec le même statut que COMMANDE, même quand la commande dépasse le délai
timeout-help-signal = spécifier le signal à envoyer en cas de délai dépassé ; SIGNAL peut être un nom comme 'HUP' ou un nombre ; voir 'kill -l' pour une liste des signaux
timeout-help-verbose = diagnostiquer vers stderr tout signal envoyé lors d'un dépassement de délai
timeout-help-duration = un nombre à virgule flottante avec un suffixe facultatif : 's' pour les secondes (par défaut), 'm' pour les minutes, 'h' pour les heures ou 'd' pour les jours ; une durée de 0 désactive le délai d'expiration associé
timeout-help-command = une commande à exécuter avec des arguments optionels
timeout-after-help = À l'expiration du délai, le signal TERM est envoyé à COMMANDE, si aucun autre SIGNAL n'est spécifié. Le signal TERM tue tout processus qui ne bloque pas ou n'intercepte pas ce signal. Il peut être nécessaire d'utiliser le signal KILL, puisque ce signal ne peut pas être intercepté.

  Statut de sortie :
    124  si COMMANDE expire et que --preserve-status n'est pas spécifié
    125  si la commande timeout elle-même échoue
    126  si COMMANDE est trouvé mais ne peut être invoqué
    127  si COMMANDE est introuvable
    137  si COMMANDE (ou timeout lui-même) reçoit le signal KILL (9) (128+9)
    -    sinon, le statut de sortie de COMMANDE

# Messages d'erreur
timeout-error-invalid-signal = { $signal } : signal invalide
timeout-error-failed-to-execute-process = échec d'exécution du processus : { $error }

# Messages détaillés
timeout-verbose-sending-signal = envoi du signal { $signal } à la commande { $command }
