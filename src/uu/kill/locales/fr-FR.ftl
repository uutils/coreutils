kill-about = Envoyer un signal aux processus ou lister les informations sur les signaux.
kill-usage = kill [OPTIONS]... PID...

# Messages d'aide
kill-help-list = Liste les signaux
kill-help-table = Liste le tableau des signaux
kill-help-signal = Envoie le signal donné au lieu de SIGTERM

# Messages d'erreur
kill-error-no-process-id = aucun ID de processus spécifié
  Essayez --help pour plus d'informations.
kill-error-invalid-signal = { $signal } : signal invalide
kill-error-parse-argument = échec de l'analyse de l'argument { $argument } : { $error }
kill-error-sending-signal = échec de l'envoi du signal au processus { $pid }
