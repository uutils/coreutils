chmod-about = Changer le mode de chaque FICHIER vers MODE.
  Avec --reference, changer le mode de chaque FICHIER vers celui de RFICHIER.
chmod-usage = chmod [OPTION]... MODE[,MODE]... FICHIER...
  chmod [OPTION]... MODE-OCTAL FICHIER...
  chmod [OPTION]... --reference=RFICHIER FICHIER...
chmod-after-help = Chaque MODE est de la forme [ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+.

# Messages d'aide
chmod-help-print-help = Afficher les informations d'aide.
chmod-help-changes = comme verbeux mais rapporter seulement lors d'un changement
chmod-help-quiet = supprimer la plupart des messages d'erreur
chmod-help-verbose = afficher un diagnostic pour chaque fichier traité
chmod-help-no-preserve-root = ne pas traiter '/' spécialement (par défaut)
chmod-help-preserve-root = échouer à opérer récursivement sur '/'
chmod-help-recursive = changer les fichiers et répertoires récursivement
chmod-help-reference = utiliser le mode de RFICHIER au lieu des valeurs de MODE

# Messages d'erreur
chmod-error-cannot-stat = impossible d'obtenir les attributs de {$file}
chmod-error-dangling-symlink = impossible d'opérer sur le lien symbolique pendouillant {$file}
chmod-error-no-such-file = impossible d'accéder à {$file} : Aucun fichier ou répertoire de ce type
chmod-error-preserve-root = il est dangereux d'opérer récursivement sur {$file}
  chmod: utiliser --no-preserve-root pour outrepasser cette protection
chmod-error-permission-denied = impossible d'accéder à {$file} : Permission refusée
chmod-error-new-permissions = {$file} : les nouvelles permissions sont {$actual}, pas {$expected}
chmod-error-missing-operand = opérande manquant

# Messages verbeux/de statut
chmod-verbose-failed-dangling = échec du changement de mode de {$file} de 0000 (---------) vers 1500 (r-x-----T)
chmod-verbose-neither-changed = ni le lien symbolique {$file} ni la référence n'ont été changés
chmod-verbose-mode-retained = mode de {$file} conservé comme {$mode_octal} ({$mode_display})
chmod-verbose-failed-change = échec du changement de mode du fichier {$file} de {$old_mode} ({$old_mode_display}) vers {$new_mode} ({$new_mode_display})
chmod-verbose-mode-changed = mode de {$file} changé de {$old_mode} ({$old_mode_display}) vers {$new_mode} ({$new_mode_display})
