# La page des erreurs coûteuses sans ses styles

**Priorité : haute — l'écran d'un jet irréversible est illisible**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`assets/static/css/pages/costly-mistakes.css`,
`tests/e2e/test_erreurs_couteuses.py`

## L'objectif

La page « Erreurs coûteuses » a l'aspect de ses voisines : ses trois panneaux
sont des cartes blanches, son encart d'avertissement est un bandeau teinté, son
bouton de sortie est un bouton, et la tranche de trésorerie du coach est mise en
évidence dans la table.

## Ce qui l'a fait naître

Signalé par l'utilisateur : « le CSS de la page ne s'applique pas du tout ».

La feuille est bien construite et bien servie — inscrite au bundle, présente
dans le fichier rendu, chargée par le layout, et sa racine `.costly-mistakes`
est bien celle du gabarit. Ce sont les **noms** qui ne se rencontrent pas.

**Huit tokens de couleur qui n'existent pas dans cette portée.** C'est le plus
lourd des trois. La feuille emploie `--role-action`, `--role-risk`,
`--role-caution`, `--role-safe` et leurs quatre teintes. `common.css` n'en
porte aucun ; `dis-page.css` et `rec-page.css` les déclarent **chacune sous sa
propre racine**, et `--role-safe` comme `--tint-safe` ne vivent que dans la
maquette. Sous `.costly-mistakes`, les huit sont indéfinis.

Un `var()` indéfini invalide la déclaration entière. Le bouton du jet perdait
donc son fond et gardait son texte blanc : invisible sur un panneau blanc. Le
liseré rouge de l'en-tête, le badge de phase, la tranche mise en évidence et
les quatre verdicts tombaient de la même façon.

**Huit classes employées sans aucune règle dans la portée.** Le gabarit reprend
celles de la page de renvois — `alert`, `alert--info`, `panel`, `panel-title`,
`panel-sub`, `meta-label`, `meta-value`, `cta-primary` — et l'en-tête de la
feuille affirme qu'elles « vivent déjà dans le global ». C'est faux : elles
sont définies sous `.dis-page` et `.rec-page`, chacune dans sa propre portée.
Sous `.costly-mistakes`, aucune ne s'applique. D'où des panneaux sans fond, un
encart sans bandeau et un lien de sortie sans bouton.

**Deux noms qui ne correspondent pas.** Le partiel de table pose
`cm-band--current` et `cm-cell--hit` ; la feuille style `tr.current` et
`td.hit`. La tranche du coach n'est donc jamais mise en évidence, ni la colonne
atteinte par le dé — précisément les deux choses que la table existe pour
montrer.

**Une classe sans règle nulle part** : `cm-roll-error`, le message d'un jet en
échec.

Les trois se cumulent, et c'est pourquoi la page paraissait entièrement nue
alors que sa feuille était correcte à la lettre : inscrite au bundle, présente
dans le fichier rendu, chargée par le layout, et scopée sous la bonne racine.

## Ce qu'aucun verrou ne voyait

Trois contrôles encadrent le CSS, et le défaut passe entre eux :

| Contrôle | Ce qu'il vérifie |
|---|---|
| axe 14 | la feuille est inscrite au bundle |
| axe 17 | une règle ne style pas du markup vivant **hors** de sa racine |
| `check-css-collisions.sh` | les règles **portent** leur portée |

Aucun ne vérifie le sens inverse : **du markup qui emploie une classe qu'aucune
règle de sa portée ne définit**. L'axe 17 documente d'ailleurs pourquoi la
variante au navigateur a été écartée — 68 % des sélecteurs ne rencontrent rien
sur le harnais, un bruit qu'un verrou ne ferait pas lire.

Le contrôle inverse, lui, serait peu bruyant : il part du gabarit, pas de la
feuille. Il n'est pas dans cette carte, qui répare l'écran ; il mérite d'être
pesé à part.

## Le changement

**Les huit tokens sont déclarés sous `.costly-mistakes`**, copiés de la maquette
d'origine, qui les porte tous les huit sur son `:root`. Les six que
`dis-page.css` partage y ont exactement les mêmes valeurs — vérifié, aucune
divergence à arbitrer.

**Les huit règles sont copiées de `dis-page.css`**, à l'identique, seule la
portée change. C'est la page dont l'en-tête de cette feuille dit déjà s'être
inspirée. Seules les variantes réellement employées sont reprises :
`alert--warn`, `alert--error`, `meta-value--alert` et
`cta-primary--destructive` restent chez leur propriétaire, faute de markup ici.

**Les deux sélecteurs de table prennent le nom du markup**, et non l'inverse :
`cm-band--current` et `cm-cell--hit` suivent la convention `cm-` de toute la
feuille, quand `current` et `hit` sont des noms nus qu'un autre écran pourrait
revendiquer.

**`cm-roll-error` reçoit la forme d'un bandeau d'erreur**, reprise de
`alert--error` de la même page de renvois.

**L'en-tête de la feuille est corrigé** : il affirmait que ces classes vivaient
dans le global, ce qui est exactement ce qui a fait croire le travail fini.

## Ce que la carte ne touche pas

`cm-header-logo` reste une règle sans markup : la maquette porte un carré aux
initiales de l'équipe, le gabarit livré n'en a pas. Lui en donner un demanderait
de faire descendre les initiales jusqu'au view model, ce qui est une décision
d'écran, pas une correction de style.

## Tests

E2E, `test_3_la_page_porte_ses_styles` dans `test_erreurs_couteuses.py` : les
deux tokens se résolvent, les panneaux ont un fond, le bouton du jet a un fond,
et la tranche courante est mise en évidence. **La mesure passe par les styles
calculés dans le navigateur, jamais par la présence d'une classe** — c'est
précisément une classe présente et sans effet qui a produit le défaut, et un
test qui chercherait `.panel` dans le DOM aurait été vert tout du long.

Il est inséré avant le jet, seul moment où la page est consultable : le dé fait
repasser l'équipe en `ReadyToPlay`, et l'écran répond alors 422. Les tests
suivants sont renumérotés.

## Terminé quand

La page ressemble à la maquette `rawpages/app-team-costly-mistakes.html` : trois
cartes blanches, un bandeau bleuté, un bouton plein, et la tranche du coach
surlignée dans la table.
