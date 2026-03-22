timeout-about = Démarrer COMMANDE, et la tuer si elle fonctionne encore après DURÉE.
timeout-usage = timeout [OPTION] DURÉE COMMANDE...

# Messages d'aide
timeout-help-foreground = quand on n'exécute pas timeout directement depuis une invite de shell, permettre à COMMANDE de lire depuis le TTY et d'obtenir les signaux TTY ; dans ce mode, les enfants de COMMANDE ne seront pas limités dans le temps
timeout-help-kill-after = envoyer aussi un signal KILL si COMMANDE fonctionne encore si longtemps après que le signal initial ait été envoyé
timeout-help-preserve-status = sortir avec le même statut que COMMANDE, même quand la commande dépasse le délai
timeout-help-signal = spécifier le signal à envoyer en cas de délai dépassé ; SIGNAL peut être un nom comme 'HUP' ou un nombre ; voir 'kill -l' pour une liste des signaux
timeout-help-verbose = diagnostiquer vers stderr tout signal envoyé lors d'un dépassement de délai

# Messages d'erreur
timeout-error-invalid-signal = { $signal } : signal invalide
timeout-error-failed-to-execute-process = échec d'exécution du processus : { $error }

# Messages détaillés
timeout-verbose-sending-signal = envoi du signal { $signal } à la commande { $command }
