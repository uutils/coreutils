fmt-about = Reformate les paragraphes depuis l'entrée (ou l'entrée standard) vers la sortie standard.
fmt-usage = [OPTION]... [FICHIER]...

# Messages d'aide
fmt-crown-margin-help = La première et la deuxième ligne d'un paragraphe peuvent avoir des indentations différentes, auquel cas l'indentation de la première ligne est préservée, et chaque ligne suivante correspond à l'indentation de la deuxième ligne.
fmt-tagged-paragraph-help = Comme -c, sauf que la première et la deuxième ligne d'un paragraphe *doivent* avoir des indentations différentes ou elles sont traitées comme des paragraphes séparés.
fmt-preserve-headers-help = Tente de détecter et préserver les en-têtes de courrier dans l'entrée. Attention en combinant ce drapeau avec -p.
fmt-split-only-help = Divise les lignes seulement, ne les reformate pas.
fmt-uniform-spacing-help = Insère exactement un espace entre les mots, et deux entre les phrases. Les fins de phrase dans l'entrée sont détectées comme [?!.] suivies de deux espaces ou d'une nouvelle ligne ; les autres ponctuations ne sont pas interprétées comme des fins de phrase.
fmt-prefix-help = Reformate seulement les lignes commençant par PRÉFIXE, en rattachant PRÉFIXE aux lignes reformatées. À moins que -x soit spécifié, les espaces de début seront ignorés lors de la correspondance avec PRÉFIXE.
fmt-skip-prefix-help = Ne reformate pas les lignes commençant par PSKIP. À moins que -X soit spécifié, les espaces de début seront ignorés lors de la correspondance avec PSKIP
fmt-exact-prefix-help = PRÉFIXE doit correspondre au début de la ligne sans espace précédent.
fmt-exact-skip-prefix-help = PSKIP doit correspondre au début de la ligne sans espace précédent.
fmt-width-help = Remplit les lignes de sortie jusqu'à un maximum de WIDTH colonnes, par défaut 75. Cela peut être spécifié comme un nombre négatif dans le premier argument.
fmt-goal-help = Largeur objectif, par défaut 93% de WIDTH. Doit être inférieur ou égal à WIDTH.
fmt-quick-help = Divise les lignes plus rapidement au détriment d'un aspect potentiellement plus irrégulier.
fmt-tab-width-help = Traite les tabulations comme TABWIDTH espaces pour déterminer la longueur de ligne, par défaut 8. Notez que ceci n'est utilisé que pour calculer les longueurs de ligne ; les tabulations sont préservées dans la sortie.

# Messages d'erreur
fmt-error-invalid-goal = objectif invalide : {$goal}
fmt-error-goal-greater-than-width = GOAL ne peut pas être supérieur à WIDTH.
fmt-error-invalid-width = largeur invalide : {$width}
fmt-error-width-out-of-range = largeur invalide : '{$width}' : Résultat numérique hors limites
fmt-error-invalid-tabwidth = Spécification TABWIDTH invalide : {$tabwidth}
fmt-error-first-option-width = option invalide -- {$option} ; -WIDTH n'est reconnu que lorsqu'il est la première
  option ; utilisez -w N à la place
  Essayez 'fmt --help' pour plus d'informations.
fmt-error-read = erreur de lecture
fmt-error-invalid-width-malformed = largeur invalide : {$width}
fmt-error-cannot-open-for-reading = impossible d'ouvrir {$file} en lecture
fmt-error-cannot-get-metadata = impossible d'obtenir les métadonnées pour {$file}
fmt-error-failed-to-write-output = échec de l'écriture de sortie
