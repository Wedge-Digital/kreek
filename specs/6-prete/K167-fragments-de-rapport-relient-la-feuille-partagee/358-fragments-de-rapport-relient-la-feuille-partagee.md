# `match_report` — quatre fragments rechargent la feuille partagée

**Priorité : basse** — aucun effet visible, quatre chargements de trop
**Trouvée par :** le lot 2 de la carte 341, en cherchant la cause d'une
régression de rendu
**Fichiers :** les quatre `*-widget.html` de `match_report/io/web/templates/`

## Le problème

Sur la page des actions de match, `match-report-shared.css` est chargée
**quatre fois** :

```
match-report-shared.css → match-report-actions.css → match-report-shared.css
                                                   → match-report-shared.css
                                                   → match-report-shared.css
```

La première par la page, les trois autres par les fragments injectés en HTMX —
panneau d'action, journal, sélecteur de tour — qui portent chacun leur propre
`<link>` vers elle. Le quatrième fragment, le sélecteur de joueurs temporaires,
fait de même sur l'étape 4.

Ces fragments ne sont servis que sous `/step3/…` et `/step4/…`, c'est-à-dire
uniquement dans une page qui charge déjà la feuille. Leur `<link>` est donc
redondant.

Il explique aussi les cinq feuilles **vides** de `widgets/` — `action-log.css`,
`action-panel.css`, `turn-selector.css`, `temp-player-selector.css`,
`match-player-selector.css` — qui ne contiennent qu'un commentaire renvoyant à
`match-report-shared.css` : les styles de ces fragments vivent dans la feuille
partagée, d'où le rechargement.

## Ce que la carte croyait corriger, et ne corrige pas

Cette carte est née d'une hypothèse **fausse**. Le lot 2 de la carte 341 avait
buté sur une régression : scoper les feuilles `match-report-*` déplaçait un
`gap` de 24 px à 12 px et un `padding` de 6 px à 4 px. L'explication avancée
était que les `<link>` dupliqués plaçaient la feuille partagée *après* celle de
la page, lui donnant la main à spécificité égale.

**Mesuré : retirer les quatre `<link>` ne change aucune valeur calculée**, sur
78 702 relevés. La duplication n'arbitrait donc rien, et la régression du lot 2
a une autre cause — qui reste à trouver.

La carte garde sa valeur : quatre chargements d'une feuille de 250 lignes en
moins à chaque affichage d'une page d'actions, et une duplication de moins
avant la fusion de la carte 342. Mais elle ne débloque pas la 341.

## Ce que ça implique

Les quatre fragments dépendent désormais de leur page hôte pour leurs styles.
C'est déjà le cas en pratique — ils ne sont servis que par elle — mais ça cesse
d'être vrai par construction. Si l'un d'eux devait un jour être rendu ailleurs,
il faudrait lui rendre son `<link>`, ou déplacer ses règles dans sa propre
feuille, aujourd'hui vide.

## Checklist

- [x] Les quatre fragments ne portent plus de `<link>` vers la feuille partagée
- [x] Seules les cinq pages la chargent, une fois chacune
- [x] Vérifié au harnais visuel : **aucune propriété rendue ne change** sur
      86 vues et 78 702 relevés
- [ ] Les cinq feuilles vides de `widgets/` restent des marque-pages sans
      contenu — à traiter dans la carte de nettoyage prévue par l'épic E03
