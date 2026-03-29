stty-usage =
    stty [-F PÉRIPHÉRIQUE | --file=PÉRIPHÉRIQUE] [PARAMÈTRE]...
    ou :  stty [-F PÉRIPHÉRIQUE | --file=PÉRIPHÉRIQUE] [-a|--all]
    ou :  stty [-F PÉRIPHÉRIQUE | --file=PÉRIPHÉRIQUE] [-g|--save]
stty-about = Afficher ou modifier les caractéristiques du terminal.
stty-option-all = afficher tous les paramètres actuels sous forme lisible
stty-option-save = afficher tous les paramètres actuels sous forme lisible par stty
stty-option-file = ouvrir et utiliser le PÉRIPHÉRIQUE spécifié au lieu de stdin
stty-option-settings = paramètres à modifier
stty-error-options-mutually-exclusive = les options pour les styles de sortie verbeux et lisible par stty s'excluent mutuellement
stty-error-output-style-no-modes = lors de la spécification d'un style de sortie, les modes ne peuvent pas être définis
stty-error-missing-argument = argument manquant pour '{ $arg }'
stty-error-invalid-speed = { $arg } invalide '{ $speed }'
stty-error-invalid-argument = argument invalide '{ $arg }'
stty-error-invalid-integer-argument = argument entier invalide : { $value }
stty-error-invalid-integer-argument-value-too-large = argument entier invalide : { $value } : Valeur trop grande pour le type de données défini
# Chaînes de format de sortie
stty-output-speed = vitesse { $speed } bauds ;
stty-output-rows-columns = lignes { $rows } ; colonnes { $columns } ;
stty-output-line = ligne = { $line } ;
stty-output-undef = <indéfini>
stty-output-min-time = min = { $min } ; temps = { $time } ;
