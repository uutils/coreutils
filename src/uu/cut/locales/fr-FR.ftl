cut-about = Affiche les colonnes d'octets ou de champs spécifiées de chaque ligne de stdin ou des fichiers d'entrée
cut-usage = cut OPTION... [FICHIER]...
cut-after-help = Chaque appel doit spécifier un mode (quoi utiliser pour les colonnes),
  une séquence (quelles colonnes afficher), et fournir une source de données

  ### Spécifier un mode

  Utilisez --bytes (-b) ou --characters (-c) pour spécifier le mode octet

  Utilisez --fields (-f) pour spécifier le mode champ, où chaque ligne est divisée en
  champs identifiés par un caractère délimiteur. Par exemple pour un CSV typique
  vous pourriez utiliser ceci en combinaison avec la définition de la virgule comme délimiteur

  ### Spécifier une séquence

  Une séquence est un groupe de 1 ou plusieurs nombres ou plages inclusives séparés
  par des virgules.

  cut -f 2,5-7 quelque_fichier.txt

  affichera les 2ème, 5ème, 6ème, et 7ème champs pour chaque ligne source

  Les plages peuvent s'étendre jusqu'à la fin de la ligne en excluant le second nombre

  cut -f 3- quelque_fichier.txt

  affichera le 3ème champ et tous les champs suivants pour chaque ligne source

  Le premier nombre d'une plage peut être exclu, et ceci est effectivement
  identique à utiliser 1 comme premier nombre : cela fait commencer la plage à la
  première colonne. Les plages peuvent aussi afficher une seule colonne

  cut -f 1,3-5 quelque_fichier.txt

  affichera les 1er, 3ème, 4ème, et 5ème champs pour chaque ligne source

  L'option --complement, quand utilisée, inverse l'effet de la séquence

  cut --complement -f 4-6 quelque_fichier.txt

  affichera tous les champs sauf les 4ème, 5ème, et 6ème

  ### Spécifier une source de données

  Si aucun argument de fichier source n'est spécifié, stdin est utilisé comme source
  de lignes à afficher

  Si des arguments de fichier source sont spécifiés, stdin est ignoré et tous les fichiers sont
  lus consécutivement si un fichier source n'est pas lu avec succès, un avertissement
  sera affiché sur stderr, et le code de statut final sera 1, mais cut
  continuera à lire les fichiers sources suivants

  Pour afficher les colonnes depuis STDIN et un argument de fichier, utilisez - (tiret) comme
  argument de fichier source pour représenter stdin.

  ### Options du Mode Champ

  Les champs dans chaque ligne sont identifiés par un délimiteur (séparateur)

  #### Définir le délimiteur

  Définissez le délimiteur qui sépare les champs dans le fichier en utilisant l'option
  --delimiter (-d). Définir le délimiteur est optionnel.
  Si non défini, un délimiteur par défaut de Tab sera utilisé.

  Si l'option -w est fournie, les champs seront séparés par tout nombre
  de caractères d'espacement (Espace et Tab). Le délimiteur de sortie sera
  un Tab sauf si explicitement spécifié. Seulement une des options -d ou -w peut être spécifiée.
  Ceci est une extension adoptée de FreeBSD.

  #### Filtrage optionnel basé sur le délimiteur

  Si le drapeau --only-delimited (-s) est fourni, seules les lignes qui
  contiennent le délimiteur seront affichées

  #### Remplacer le délimiteur

  Si l'option --output-delimiter est fournie, l'argument utilisé pour
  elle remplacera le caractère délimiteur dans chaque ligne affichée. Ceci est
  utile pour transformer les données tabulaires - par ex. pour convertir un CSV en
  TSV (fichier séparé par tabulations)

  ### Fins de ligne

  Quand l'option --zero-terminated (-z) est utilisée, cut voit \\0 (null) comme le
  caractère de 'fin de ligne' (à la fois pour lire les lignes et
  séparer les lignes affichées) au lieu de \\n (nouvelle ligne). Ceci est utile pour
  les données tabulaires où certaines cellules peuvent contenir des nouvelles lignes

  echo 'ab\\0cd' | cut -z -c 1

  donnera comme résultat 'a\\0c\\0'

# Messages d'aide
cut-help-bytes = filtrer les colonnes d'octets depuis la source d'entrée
cut-help-characters = alias pour le mode caractère
cut-help-delimiter = spécifier le caractère délimiteur qui sépare les champs dans la source d'entrée. Par défaut Tab.
cut-help-whitespace-delimited = Utiliser tout nombre d'espaces (Espace, Tab) pour séparer les champs dans la source d'entrée (extension FreeBSD).
cut-help-fields = filtrer les colonnes de champs depuis la source d'entrée
cut-help-complement = inverser le filtre - au lieu d'afficher seulement les colonnes filtrées, afficher toutes sauf ces colonnes
cut-help-only-delimited = en mode champ, afficher seulement les lignes qui contiennent le délimiteur
cut-help-zero-terminated = au lieu de filtrer les colonnes basées sur la ligne, filtrer les colonnes basées sur \\0 (caractère NULL)
cut-help-output-delimiter = en mode champ, remplacer le délimiteur dans les lignes de sortie avec l'argument de cette option

# Messages d'erreur
cut-error-is-directory = Est un répertoire
cut-error-write-error = erreur d'écriture
cut-error-delimiter-and-whitespace-conflict = entrée invalide : Seulement une des options --delimiter (-d) ou -w peut être spécifiée
cut-error-delimiter-must-be-single-character = le délimiteur doit être un caractère unique
cut-error-multiple-mode-args = usage invalide : attend au plus une des options --fields (-f), --chars (-c) ou --bytes (-b)
cut-error-missing-mode-arg = usage invalide : attend une des options --fields (-f), --chars (-c) ou --bytes (-b)
cut-error-delimiter-only-with-fields = entrée invalide : L'option '--delimiter' ('-d') n'est utilisable que si on affiche une séquence de champs
cut-error-whitespace-only-with-fields = entrée invalide : L'option '-w' n'est utilisable que si on affiche une séquence de champs
cut-error-only-delimited-only-with-fields = entrée invalide : L'option '--only-delimited' ('-s') n'est utilisable que si on affiche une séquence de champs
