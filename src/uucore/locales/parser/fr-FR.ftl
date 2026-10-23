# Analyseur partagé d'arguments GNU
parser-error-unknown-option = option non reconnue { $option }
parser-error-ambiguous-option = l'option { $option } est ambiguë ; possibilités : { $candidates }
parser-error-missing-value = l'option { $option } nécessite un argument
parser-error-unexpected-value = l'option { $option } ne permet pas d'argument
parser-error-invalid-value = { $error_word } : valeur invalide { $value } pour { $option }
