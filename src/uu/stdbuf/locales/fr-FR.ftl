stdbuf-about = Exécute COMMANDE, avec des opérations de mise en mémoire tampon modifiées pour ses flux standards.

  Les arguments obligatoires pour les options longues le sont aussi pour les options courtes.
stdbuf-usage = stdbuf [OPTION]... COMMANDE
stdbuf-after-help = Si MODE est 'L', le flux correspondant sera mis en mémoire tampon par ligne.
  Cette option n'est pas valide avec l'entrée standard.

  Si MODE est '0', le flux correspondant ne sera pas mis en mémoire tampon.

  Sinon, MODE est un nombre qui peut être suivi par l'un des suivants :

  KB 1000, K 1024, MB 1000*1000, M 1024*1024, et ainsi de suite pour G, T, P, E, Z, Y.
  Dans ce cas, le flux correspondant sera entièrement mis en mémoire tampon avec la taille de tampon définie à MODE octets.

  NOTE : Si COMMANDE ajuste la mise en mémoire tampon de ses flux standards (tee le fait par exemple), cela remplacera les paramètres correspondants modifiés par stdbuf.
  De plus, certains filtres (comme dd et cat etc.) n'utilisent pas de flux pour les E/S, et ne sont donc pas affectés par les paramètres stdbuf.

stdbuf-help-input = ajuster la mise en mémoire tampon du flux d'entrée standard
stdbuf-help-output = ajuster la mise en mémoire tampon du flux de sortie standard
stdbuf-help-error = ajuster la mise en mémoire tampon du flux d'erreur standard
stdbuf-value-mode = MODE

stdbuf-error-line-buffering-stdin-meaningless = la mise en mémoire tampon par ligne de stdin n'a pas de sens
stdbuf-error-invalid-mode = mode invalide {$error}
stdbuf-error-value-too-large = mode invalide '{$value}' : Valeur trop grande pour le type de données défini
stdbuf-error-command-not-supported = Commande non prise en charge pour ce système d'exploitation !
stdbuf-error-external-libstdbuf-not-found = libstdbuf externe introuvable au chemin configuré : {$path}
stdbuf-error-permission-denied = échec de l'exécution du processus : Permission refusée
stdbuf-error-no-such-file = échec de l'exécution du processus : Aucun fichier ou répertoire de ce type
stdbuf-error-failed-to-execute = échec de l'exécution du processus : {$error}
stdbuf-error-killed-by-signal = processus tué par le signal {$signal}
