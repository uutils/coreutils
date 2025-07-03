pr-about = paginer ou présenter en colonnes le(s) FICHIER(s) pour l'impression
pr-after-help =
  Si aucun FICHIER n'est donné, ou si FICHIER est -, lire l'entrée standard.

  Lors de la création d'une sortie multi-colonnes, les colonnes auront une largeur égale.
  Lors de l'utilisation de l'option '-s' pour séparer les colonnes, le séparateur par défaut
  est un caractère de tabulation unique. Lors de l'utilisation de l'option '-S' pour séparer
  les colonnes, le séparateur par défaut est un caractère d'espace unique.
pr-usage = pr [OPTION]... [FICHIER]...

# Messages d'aide
pr-help-pages = Commencer et arrêter l'impression à la page PREMIÈRE_PAGE[:DERNIÈRE_PAGE]
pr-help-header =
  Utiliser la chaîne d'en-tête pour remplacer le nom de fichier
                  dans la ligne d'en-tête.
pr-help-double-space =
  Produire une sortie avec double espacement. Un caractère <saut de ligne>
                  supplémentaire est affiché après chaque <saut de ligne> trouvé dans l'entrée.
pr-help-number-lines =
  Fournir une numérotation de ligne avec largeur de chiffres. La valeur par défaut
                  pour la largeur, si non spécifiée, est 5. Le numéro occupe les premières
                  largeur positions de colonne de chaque colonne de texte ou de chaque ligne
                  de sortie -m. Si char (tout caractère non numérique) est donné, il est
                  ajouté au numéro de ligne pour le séparer de ce qui suit. La valeur par
                  défaut pour char est une <tabulation>. Les numéros de ligne plus longs
                  que largeur colonnes sont tronqués.
pr-help-first-line-number = commencer le comptage avec NUMÉRO à la 1ère ligne de la première page imprimée
pr-help-omit-header =
  N'écrire ni l'en-tête d'identification de cinq lignes ni le pied de page
                  de cinq lignes habituellement fournis pour chaque page. Arrêter l'écriture
                  après la dernière ligne de chaque fichier sans espacement jusqu'à la fin de la page.
pr-help-page-length =
  Remplacer la valeur par défaut de 66 lignes (nombre par défaut de lignes de texte 56,
                  et avec -F 63) et réinitialiser la longueur de page à lignes. Si lignes n'est pas
                  supérieur à la somme des profondeurs de l'en-tête et du pied de page (en lignes),
                  l'utilitaire pr supprimera à la fois l'en-tête et le pied de page, comme si l'option -t
                  était en vigueur.
pr-help-no-file-warnings = omettre l'avertissement lorsqu'un fichier ne peut pas être ouvert
pr-help-form-feed =
  Utiliser un <saut de page> pour les nouvelles pages, au lieu du comportement par défaut
                  qui utilise une séquence de <sauts de ligne>.
pr-help-column-width =
  Définir la largeur de la ligne à largeur positions de colonne pour la sortie
                  multi-colonnes de texte seulement. Si l'option -w n'est pas spécifiée et
                  l'option -s n'est pas spécifiée, la largeur par défaut sera 72. Si l'option -w
                  n'est pas spécifiée et l'option -s est spécifiée, la largeur par défaut sera 512.
pr-help-page-width =
  définir la largeur de page à PAGE_WIDTH (72) caractères toujours,
                  tronquer les lignes, sauf si l'option -J est définie, aucune interférence
                  avec -S ou -s
pr-help-across =
  Modifier l'effet de l'option - colonne pour que les colonnes soient remplies
                  à travers la page dans un ordre round-robin (par exemple, quand colonne est 2,
                  la première ligne d'entrée va en tête de colonne 1, la seconde va en tête de colonne 2,
                  la troisième est la seconde ligne en colonne 1, et ainsi de suite).
pr-help-column =
  Produire une sortie multi-colonnes qui est arrangée en colonnes colonnes
                  (la valeur par défaut sera 1) et est écrite dans chaque colonne dans l'ordre
                  dans lequel le texte est reçu du fichier d'entrée. Cette option ne doit pas être
                  utilisée avec -m. Les options -e et -i seront assumées pour la sortie multi-colonnes
                  de texte. Que les colonnes de texte soient produites avec des longueurs verticales
                  identiques ou non n'est pas spécifié, mais une colonne de texte ne doit jamais
                  dépasser la longueur de la page (voir l'option -l). Lorsqu'utilisée avec -t,
                  utiliser le nombre minimum de lignes pour écrire la sortie.
pr-help-column-char-separator =
  Séparer les colonnes de texte par le caractère unique char au lieu du nombre
                  approprié d'<espaces> (par défaut pour char est le caractère de <tabulation>).
pr-help-column-string-separator =
  séparer les colonnes par CHAÎNE,
                  sans -S : Séparateur par défaut <TAB> avec -J et <espace>
                  sinon (même que -S\" \"), aucun effet sur les options de colonne
pr-help-merge =
  Fusionner les fichiers. La sortie standard sera formatée pour que l'utilitaire pr
                  écrive une ligne de chaque fichier spécifié par un opérande de fichier, côte à côte
                  dans des colonnes de texte de largeurs fixes égales, en termes du nombre de positions
                  de colonne. Les implémentations doivent supporter la fusion d'au moins neuf opérandes de fichier.
pr-help-indent =
  Chaque ligne de sortie sera précédée par décalage <espaces>. Si l'option -o
                  n'est pas spécifiée, le décalage par défaut sera zéro. L'espace pris est
                  en plus de la largeur de ligne de sortie (voir l'option -w ci-dessous).
pr-help-join-lines =
  fusionner les lignes complètes, désactive la troncature de ligne -W, aucun
                  alignement de colonne, --sep-string[=CHAÎNE] définit les séparateurs
pr-help-help = Afficher les informations d'aide

# Texte d'en-tête de page
pr-page = Page

# Messages d'erreur
pr-error-reading-input = pr : La lecture depuis l'entrée {$file} a donné une erreur
pr-error-unknown-filetype = pr : {$file} : type de fichier inconnu
pr-error-is-directory = pr : {$file} : Est un répertoire
pr-error-socket-not-supported = pr : impossible d'ouvrir {$file}, Opération non supportée sur socket
pr-error-no-such-file = pr : impossible d'ouvrir {$file}, Aucun fichier ou répertoire de ce type
pr-error-column-merge-conflict = impossible de spécifier le nombre de colonnes lors de l'impression en parallèle
pr-error-across-merge-conflict = impossible de spécifier à la fois l'impression transversale et l'impression en parallèle
pr-error-invalid-pages-range = argument --pages invalide '{$start}:{$end}'
