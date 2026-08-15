# Chaînes que seul un chemin d'erreur demande : les messages des analyseurs
# partagés et les étiquettes portées par un caret. Elles sont séparées des
# chaînes uucore communes, que chaque utilitaire analyse au démarrage alors
# que presque aucune exécution n'en a besoin.

checksum-error-failed-to-read-input = échec de la lecture de l'entrée
checksum-error-algo-bad-format = { $file }: { $line }: ligne invalide pour { $algo }
# Messages d'analyse des chaînes de format (printf, seq, env, ...)
format-error-invalid-spec = %{ $spec } : spécification de conversion invalide
format-error-too-many-specs = le format '{ $format }' a trop de directives %
format-error-no-spec = le format '{ $format }' n'a pas de directive %
format-error-ends-with-percent = le format { $format } se termine par %
format-error-invalid-precision = précision invalide : '{ $precision }'
format-error-wrong-spec-type = type de directive % incorrect
format-error-write = erreur d'écriture : { $error }
format-error-no-more-arguments = plus d'arguments
format-error-invalid-argument = argument invalide
format-error-missing-hex = nombre hexadécimal manquant dans l'échappement
format-error-invalid-universal-character = nom de caractère universel invalide \{ $escape }{ $digits }

# Le mot en tête de la ligne de conseil d'un rapport avec caret
diagnostics-help-label = Aide{" "}

# Étiquette de diagnostic partagée par les utilitaires dont les arguments forment une expression
diagnostics-label-expression-complete = l'expression était déjà complète ici

# Erreurs de somme de contrôle (cksum, md5sum, sha*sum, b2sum)
checksum-error-raw-multiple-files = l'option --raw n'est pas prise en charge avec plusieurs fichiers
checksum-error-check-only-flag = l'option --{ $flag } n'a de sens que lors de la vérification de sommes de contrôle
checksum-error-length-required = --length est requis pour { $algorithm }
checksum-error-invalid-length = longueur invalide : { $length }
checksum-error-length-too-big-for-blake = la longueur maximale d'empreinte pour { $algorithm } est de 512 bits
checksum-error-length-not-multiple-of-8 = la longueur n'est pas un multiple de 8
checksum-error-invalid-length-for-sha = la longueur d'empreinte pour { $algorithm } doit être 224, 256, 384 ou 512
checksum-error-length-required-for-sha = --algorithm={ $algorithm } nécessite de préciser --length 224, 256, 384 ou 512
checksum-error-length-only-for-blake2b-sha2-sha3 = --length n'est pris en charge qu'avec --algorithm blake2b, sha2 ou sha3
checksum-error-binary-text-conflict = les options --binary et --text n'ont pas de sens lors de la vérification de sommes de contrôle
checksum-error-text-without-untagged = le mode --text n'est pris en charge qu'avec --untagged
checksum-error-tag-check = l'option --tag n'a pas de sens lors de la vérification de sommes de contrôle
checksum-error-text-after-tag = --tag ne prend pas en charge le mode --text
checksum-error-algorithm-not-supported-with-check = --check n'est pas pris en charge avec --algorithm={"{"}bsd,sysv,crc,crc32b{"}"}
checksum-error-combine-multiple-algorithms = Vous ne pouvez pas combiner plusieurs algorithmes de hachage !
checksum-error-need-algorithm-to-hash = Un algorithme de hachage est nécessaire.
checksum-error-unknown-algorithm = algorithme inconnu : { $algorithm } : clap aurait dû empêcher ce cas

# Étiquettes de diagnostic : ce que le caret désigne dans une TAILLE
size-diag-label-invalid-suffix = unité inconnue
size-diag-label-too-big = ce nombre est trop grand pour être utilisé
size-diag-help-syntax = une taille est un nombre suivi d'une unité facultative : K, M, G et ainsi de suite pour 1024, KB, MB, GB pour 1000

# Étiquettes de diagnostic : ce que le caret désigne dans une liste
# d'intervalles. Ce qu'une borne nulle a de faux dépend de ce que l'intervalle
# compte, donc chaque utilitaire le dit avec ses propres mots.
range-diag-label-too-large = ce nombre est trop grand
range-diag-label-inverted = cet intervalle se termine avant de commencer
