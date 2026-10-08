kill-about = Envoyer un signal aux processus ou lister les informations sur les signaux.
kill-usage = kill [OPTIONS]... PID...
kill-after-help-windows = Notes pour Windows :
  Les processus signalés sont terminés de force (Windows ne délivre pas de
  signaux) ; leur code de sortie est 128 plus le numéro du signal. Les PID
  négatifs (le groupe d'un autre processus) et STOP ne sont pas pris en charge.
  Les permissions proviennent de votre jeton actuel, avec SeDebugPrivilege
  activé lorsqu'il est détenu ; exécutez kill en tant qu'administrateur pour
  atteindre les processus qu'un jeton standard ne peut pas signaler. Les
  processus protégés (anti-programmes malveillants) ne peuvent jamais être
  terminés.

  Le PID 0 cible l'objet Job dans lequel kill s'exécute, l'équivalent Windows
  le plus proche d'un groupe de processus. Tous les processus de ce Job et de
  ses Jobs enfants sont signalés, kill lui-même en dernier, de sorte que kill
  meurt avec le groupe. Hors d'un Job, le PID 0 ne signale que kill lui-même.

  Attention : un objet Job ne vous appartient généralement pas. Les terminaux,
  les IDE, Docker, les agents d'intégration continue et l'Assistant de
  compatibilité des programmes de Windows exécutent tous dans un Job ce qu'ils
  lancent, et un Job capture chaque descendant dès sa création. Sous un agent
  d'intégration continue, kill 0 signale l'agent et toutes les étapes voisines.
  La portée peut être bien plus large que celle d'un groupe de processus POSIX.

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
kill-error-negative-pid-unsupported = un PID négatif (le groupe d'un autre processus) n'est pas pris en charge sur Windows
