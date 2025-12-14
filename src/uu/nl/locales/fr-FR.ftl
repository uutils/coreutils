nl-about = Numéroter les lignes des fichiers
nl-usage = nl [OPTION]... [FICHIER]...
nl-after-help = STYLE est l'un des suivants :

  - a numéroter toutes les lignes
  - t numéroter seulement les lignes non vides
  - n ne numéroter aucune ligne
  - pBRE numéroter seulement les lignes qui contiennent une correspondance pour
          l'expression régulière de base, BRE

  FORMAT est l'un des suivants :

  - ln justifié à gauche, sans zéros en tête
  - rn justifié à droite, sans zéros en tête
  - rz justifié à droite, avec zéros en tête

# Messages d'aide
nl-help-help = Afficher les informations d'aide.
nl-help-body-numbering = utiliser STYLE pour numéroter les lignes du corps
nl-help-section-delimiter = utiliser CC pour séparer les pages logiques
nl-help-footer-numbering = utiliser STYLE pour numéroter les lignes de pied de page
nl-help-header-numbering = utiliser STYLE pour numéroter les lignes d'en-tête
nl-help-line-increment = incrément du numéro de ligne à chaque ligne
nl-help-join-blank-lines = groupe de NUMBER lignes vides comptées comme une seule
nl-help-number-format = insérer les numéros de ligne selon FORMAT
nl-help-no-renumber = ne pas remettre à zéro les numéros de ligne aux pages logiques
nl-help-number-separator = ajouter STRING après le numéro de ligne (éventuel)
nl-help-starting-line-number = premier numéro de ligne sur chaque page logique
nl-help-number-width = utiliser NUMBER colonnes pour les numéros de ligne

# Messages d'erreur
nl-error-invalid-arguments = Arguments fournis invalides.
nl-error-could-not-read-line = impossible de lire la ligne
nl-error-could-not-write = impossible d'écrire la sortie
nl-error-line-number-overflow = débordement du numéro de ligne
nl-error-invalid-line-width = Largeur de champ de numéro de ligne invalide : ‘{ $value }’ : Résultat numérique hors limites
nl-error-invalid-regex = expression régulière invalide
nl-error-invalid-numbering-style = style de numérotation invalide : '{ $style }'
nl-error-is-directory = { $path } : Est un répertoire
