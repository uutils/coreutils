shuf-about = Mélanger l'entrée en affichant une permutation aléatoire des lignes d'entrée.
  Chaque permutation de sortie est également probable.
  Sans FICHIER, ou quand FICHIER est -, lire l'entrée standard.
shuf-usage = shuf [OPTION]... [FICHIER]
  shuf -e [OPTION]... [ARG]...
  shuf -i MIN-MAX [OPTION]...

# Messages d'aide
shuf-help-echo = traiter chaque ARG comme une ligne d'entrée
shuf-help-input-range = traiter chaque nombre de MIN à MAX comme une ligne d'entrée
shuf-help-head-count = afficher au maximum NOMBRE lignes
shuf-help-output = écrire le résultat dans FICHIER au lieu de la sortie standard
shuf-help-random-source = obtenir des octets aléatoires depuis FICHIER
shuf-help-repeat = les lignes de sortie peuvent être répétées
shuf-help-zero-terminated = le délimiteur de ligne est NUL, pas nouvelle ligne

# Messages d'erreur
shuf-error-unexpected-argument = argument inattendu { $arg } trouvé
shuf-error-failed-to-open-for-writing = échec de l'ouverture de { $file } en écriture
shuf-error-failed-to-open-random-source = échec de l'ouverture de la source aléatoire { $file }
shuf-error-read-error = erreur de lecture
shuf-error-no-lines-to-repeat = aucune ligne à répéter
shuf-error-start-exceeds-end = le début dépasse la fin
shuf-error-missing-dash = '-' manquant
shuf-error-write-failed = échec de l'écriture
