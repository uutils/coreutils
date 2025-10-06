echo-about = Affiche une ligne de texte
echo-usage = echo [OPTIONS]... [CHAÎNE]...
echo-after-help = Affiche la ou les CHAÎNE(s) sur la sortie standard.

  Si -e est activé, les séquences suivantes sont reconnues :

  - \ barre oblique inverse
  - \a alerte (BEL)
  - \b retour arrière
  - \c ne produit aucune sortie supplémentaire
  - \e échappement
  - \f saut de page
  - \n nouvelle ligne
  - \r retour chariot
  - \t tabulation horizontale
  - \v tabulation verticale
  - \0NNN octet avec valeur octale NNN (1 à 3 chiffres)
  - \xHH octet avec valeur hexadécimale HH (1 à 2 chiffres)

echo-help-no-newline = ne pas afficher la nouvelle ligne finale
echo-help-enable-escapes = activer l'interprétation des séquences d'échappement
echo-help-disable-escapes = désactiver l'interprétation des séquences d'échappement (par défaut)

echo-error-non-utf8 = Arguments non-UTF-8 fournis, mais cette plateforme ne les prend pas en charge
