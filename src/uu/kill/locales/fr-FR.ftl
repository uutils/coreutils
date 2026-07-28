kill-about = Envoyer un signal aux processus ou lister les informations sur les signaux.
kill-usage = kill [OPTIONS]... PID...
kill-after-help-windows = Notes pour Windows :
  Les processus signalés sont terminés de force (Windows ne délivre pas de
  signaux) ; leur code de sortie est 128 plus le numéro du signal. Les groupes
  de processus (PID <= 0) et STOP ne sont pas pris en charge. Les permissions
  proviennent de votre jeton actuel : kill n'active jamais SeDebugPrivilege,
  donc un kill élevé peut signaler « Permission denied » là où « taskkill /F »
  réussit.

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
kill-error-write = erreur d'écriture : { $error }
kill-error-unsupported-signal = signal non pris en charge sur Windows
kill-error-process-groups-unsupported = les groupes de processus ne sont pas pris en charge sur Windows
