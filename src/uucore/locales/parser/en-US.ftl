# Shared GNU argument scanner
parser-error-unknown-option = unrecognized option { $option }
parser-error-ambiguous-option = option { $option } is ambiguous; possibilities: { $candidates }
parser-error-missing-value = option { $option } requires an argument
parser-error-unexpected-value = option { $option } doesn't allow an argument
parser-error-invalid-value = { $error_word }: invalid value { $value } for { $option }
