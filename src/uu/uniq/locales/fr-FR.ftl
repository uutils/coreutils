uniq-about = Signaler ou omettre les lignes répétées.
uniq-usage = uniq [OPTION]... [ENTRÉE [SORTIE]]
uniq-after-help = Filtrer les lignes adjacentes correspondantes de ENTRÉE (ou l'entrée standard),
  en écrivant vers SORTIE (ou la sortie standard).
  Note : uniq ne détecte les lignes répétées que si elles sont adjacentes.
  Vous pourriez vouloir trier l'entrée d'abord, ou utiliser sort -u sans uniq.

# Messages d'aide
uniq-help-all-repeated = afficher toutes les lignes dupliquées. La délimitation se fait avec des lignes vides. [défaut : none]
uniq-help-group = afficher tous les éléments, en séparant les groupes avec une ligne vide. [défaut : separate]
uniq-help-check-chars = comparer au maximum N caractères dans les lignes
uniq-help-count = préfixer les lignes par le nombre d'occurrences
uniq-help-ignore-case = ignorer les différences de casse lors de la comparaison
uniq-help-repeated = afficher seulement les lignes dupliquées
uniq-help-skip-chars = éviter de comparer les N premiers caractères
uniq-help-skip-fields = éviter de comparer les N premiers champs
uniq-help-unique = afficher seulement les lignes uniques
uniq-help-zero-terminated = terminer les lignes avec un octet 0, pas une nouvelle ligne

# Messages d'erreur
uniq-error-write-line-terminator = Impossible d'écrire le terminateur de ligne
uniq-error-write-error = erreur d'écriture
uniq-error-read-error = erreur de lecture
uniq-error-invalid-argument = Argument invalide pour { $opt_name } : { $arg }
uniq-error-try-help = Essayez 'uniq --help' pour plus d'informations.
uniq-error-group-mutually-exclusive = --group est mutuellement exclusif avec -c/-d/-D/-u
uniq-error-group-badoption = argument invalide 'badoption' pour '--group'
  Arguments valides :
    - 'prepend'
    - 'append'
    - 'separate'
    - 'both'

uniq-error-all-repeated-badoption = argument invalide 'badoption' pour '--all-repeated'
  Arguments valides :
    - 'none'
    - 'prepend'
    - 'separate'

uniq-error-counts-and-repeated-meaningless = afficher toutes les lignes dupliquées et les nombres de répétitions n'a pas de sens
  Essayez 'uniq --help' pour plus d'informations.
uniq-error-could-not-open = Impossible d'ouvrir { $path }
