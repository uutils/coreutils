# Chaînes communes partagées entre toutes les commandes uutils
# Principalement pour clap

# Mots génériques
common-error = erreur
common-tip = conseil
common-usage = Utilisation
common-help = aide
common-version = version

# Messages d'erreur clap communs
clap-error-unexpected-argument = { $error_word } : argument inattendu '{ $arg }' trouvé
clap-error-similar-argument = { $tip_word } : un argument similaire existe : '{ $suggestion }'
clap-error-pass-as-value = { $tip_word } : pour passer '{ $arg }' comme valeur, utilisez '{ $tip_command }'
clap-error-invalid-value = { $error_word } : valeur invalide '{ $value }' pour '{ $option }'
clap-error-value-required = { $error_word } : une valeur est requise pour '{ $option }' mais aucune n'a été fournie
clap-error-possible-values = valeurs possibles
clap-error-help-suggestion = Pour plus d'informations, essayez '{ $command } --help'.
common-help-suggestion = Pour plus d'informations, essayez '--help'.

# Modèles de texte d'aide communs
help-flag-help = Afficher les informations d'aide
help-flag-version = Afficher les informations de version

# Contextes d'erreur communs
error-io = Erreur E/S
error-permission-denied = Permission refusée
error-file-not-found = Aucun fichier ou répertoire de ce type
error-invalid-argument = Argument invalide

# Actions communes
action-copying = copie
action-moving = déplacement
action-removing = suppression
action-creating = création
action-reading = lecture
action-writing = écriture
