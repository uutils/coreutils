seq-about = Afficher les nombres de PREMIER à DERNIER, par incréments d'INCRÉMENT.
seq-usage = seq [OPTION]... DERNIER
  seq [OPTION]... PREMIER DERNIER
  seq [OPTION]... PREMIER INCRÉMENT DERNIER

# Messages d'aide
seq-help-separator = Caractère séparateur (par défaut \n)
seq-help-terminator = Caractère terminateur (par défaut \n)
seq-help-equal-width = Égaliser les largeurs de tous les nombres en remplissant avec des zéros
seq-help-format = utiliser le FORMAT de nombre à virgule flottante de style printf

# Messages d'erreur
seq-error-parse = argument { $type } invalide : { $arg }
seq-error-zero-increment = valeur d'incrément zéro invalide : { $arg }
seq-error-no-arguments = opérande manquant
seq-error-format-and-equal-width = la chaîne de format ne peut pas être spécifiée lors de l'impression de chaînes de largeur égale

# Types d'erreur d'analyse
seq-parse-error-type-float = nombre à virgule flottante
seq-parse-error-type-nan = 'non-un-nombre'
