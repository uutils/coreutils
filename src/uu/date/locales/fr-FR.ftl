date-about = afficher ou définir la date système
date-usage = [OPTION]... [+FORMAT]
  date [-u|--utc|--universal] [MMDDhhmm[[CC]YY][.ss]]

  FORMAT contrôle la sortie. Les séquences interprétées sont :
  { "| Séquence | Description                                                    | Exemple                |" }
  { "| -------- | -------------------------------------------------------------- | ---------------------- |" }
  { "| %%       | un % littéral                                                  | %                      |" }
  { "| %a       | nom abrégé du jour de la semaine selon la locale             | dim                    |" }
  { "| %A       | nom complet du jour de la semaine selon la locale            | dimanche               |" }
  { "| %b       | nom abrégé du mois selon la locale                            | jan                    |" }
  { "| %B       | nom complet du mois selon la locale                           | janvier                |" }
  { "| %c       | date et heure selon la locale                                 | jeu 3 mar 23:05:25 2005|" }
  { "| %C       | siècle ; comme %Y, sauf qu'on omet les deux derniers chiffres | 20                     |" }
  { "| %d       | jour du mois                                                   | 01                     |" }
  { "| %D       | date ; identique à %m/%d/%y                                    | 12/31/99               |" }
  { "| %e       | jour du mois, rempli avec des espaces ; identique à %_d       | 3                      |" }
  { "| %F       | date complète ; identique à %Y-%m-%d                          | 2005-03-03             |" }
  { "| %g       | deux derniers chiffres de l'année du numéro de semaine ISO (voir %G) | 05                     |" }
  { "| %G       | année du numéro de semaine ISO (voir %V) ; normalement utile seulement avec %V | 2005                   |" }
  { "| %h       | identique à %b                                                 | jan                    |" }
  { "| %H       | heure (00..23)                                                 | 23                     |" }
  { "| %I       | heure (01..12)                                                 | 11                     |" }
  { "| %j       | jour de l'année (001..366)                                     | 062                    |" }
  { "| %k       | heure, remplie avec des espaces ( 0..23) ; identique à %_H    |  3                     |" }
  { "| %l       | heure, remplie avec des espaces ( 1..12) ; identique à %_I    |  9                     |" }
  { "| %m       | mois (01..12)                                                  | 03                     |" }
  { "| %M       | minute (00..59)                                                | 30                     |" }
  { "| %n       | une nouvelle ligne                                             | \\n                     |" }
  { "| %N       | nanosecondes (000000000..999999999)                            | 123456789              |" }
  { "| %p       | équivalent locale de AM ou PM ; vide si inconnu               | PM                     |" }
  { "| %P       | comme %p, mais en minuscules                                   | pm                     |" }
  { "| %q       | trimestre de l'année (1..4)                                   | 1                      |" }
  { "| %r       | heure sur 12 heures selon la locale                           | 11:11:04 PM            |" }
  { "| %R       | heure sur 24 heures et minute ; identique à %H:%M             | 23:30                  |" }
  { "| %s       | secondes depuis 1970-01-01 00:00:00 UTC                       | 1615432800             |" }
  { "| %S       | seconde (00..60)                                               | 30                     |" }
  { "| %t       | une tabulation                                                 | \\t                     |" }
  { "| %T       | heure ; identique à %H:%M:%S                                   | 23:30:30               |" }
  { "| %u       | jour de la semaine (1..7) ; 1 est lundi                       | 4                      |" }
  { "| %U       | numéro de semaine de l'année, avec dimanche comme premier jour de la semaine (00..53) | 10                     |" }
  { "| %V       | numéro de semaine ISO, avec lundi comme premier jour de la semaine (01..53) | 12                     |" }
  { "| %w       | jour de la semaine (0..6) ; 0 est dimanche                    | 4                      |" }
  { "| %W       | numéro de semaine de l'année, avec lundi comme premier jour de la semaine (00..53) | 11                     |" }
  { "| %x       | représentation de la date selon la locale                     | 03/03/2005             |" }
  { "| %X       | représentation de l'heure selon la locale                     | 23:30:30               |" }
  { "| %y       | deux derniers chiffres de l'année (00..99)                    | 05                     |" }
  { "| %Y       | année                                                          | 2005                   |" }
  { "| %z       | fuseau horaire numérique +hhmm                                | -0400                  |" }
  { "| %:z      | fuseau horaire numérique +hh:mm                               | -04:00                 |" }
  { "| %::z     | fuseau horaire numérique +hh:mm:ss                            | -04:00:00              |" }
  { "| %:::z    | fuseau horaire numérique avec : à la précision nécessaire     | -04, +05:30            |" }
  { "| %Z       | abréviation alphabétique du fuseau horaire                    | EDT                    |" }

  Par défaut, date remplit les champs numériques avec des zéros.
  Les indicateurs optionnels suivants peuvent suivre '%' :
    { "* `-` (tiret) ne pas remplir le champ" }
    { "* `_` (soulignement) remplir avec des espaces" }
    { "* `0` (zéro) remplir avec des zéros" }
    { "* `^` utiliser des majuscules si possible" }
    { "* `#` utiliser l'inverse si possible" }
  Après tout indicateur vient une largeur de champ optionnelle, comme nombre décimal ;
  puis un modificateur optionnel, qui est soit
    { "* `E` pour utiliser les représentations alternatives de la locale si disponibles, ou" }
    { "* `O` pour utiliser les symboles numériques alternatifs de la locale si disponibles." }
  Exemples :
    Convertir les secondes depuis l'époque (1970-01-01 UTC) en date

    date --date='@2147483647'

    Montrer l'heure sur la côte ouest des États-Unis (utiliser tzselect(1) pour trouver TZ)

    TZ='America/Los_Angeles' date

date-help-date = afficher l'heure décrite par CHAÎNE, pas 'maintenant'
date-help-file = comme --date ; une fois pour chaque ligne de FICHIER_DATE
date-help-iso-8601 = afficher la date/heure au format ISO 8601.
  FMT='date' pour la date seulement (par défaut),
  'hours', 'minutes', 'seconds', ou 'ns'
  pour la date et l'heure à la précision indiquée.
  Exemple : 2006-08-14T02:34:56-06:00
date-help-rfc-email = afficher la date et l'heure au format RFC 5322.
  Exemple : Mon, 14 Aug 2006 02:34:56 -0600
date-help-rfc-3339 = afficher la date/heure au format RFC 3339.
  FMT='date', 'seconds', ou 'ns'
  pour la date et l'heure à la précision indiquée.
  Exemple : 2006-08-14 02:34:56-06:00
date-help-debug = annoter la date analysée et avertir des usages douteux sur stderr
date-help-reference = afficher l'heure de dernière modification du FICHIER
date-help-set = définir l'heure décrite par CHAÎNE
date-help-set-macos = définir l'heure décrite par CHAÎNE (pas encore disponible sur mac)
date-help-set-redox = définir l'heure décrite par CHAÎNE (pas encore disponible sur redox)
date-help-universal = afficher ou définir le Temps Universel Coordonné (UTC)

date-error-invalid-date = date invalide '{$date}'
date-error-invalid-format = format invalide '{$format}' ({$error})
date-error-expected-file-got-directory = fichier attendu, répertoire obtenu {$path}
date-error-date-overflow = débordement de date '{$date}'
date-error-setting-date-not-supported-macos = la définition de la date n'est pas prise en charge par macOS
date-error-setting-date-not-supported-redox = la définition de la date n'est pas prise en charge par Redox
date-error-cannot-set-date = impossible de définir la date
date-error-extra-operand = opérande supplémentaire '{$operand}'
