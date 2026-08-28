# Page de gestion · Phase 8 : cartes kanban

**Phase 7** : `07-integration.md`

## Dix cartes, 463 à 472

| # | Carte | Ordre | Dépend de |
|---|---|---|---|
| 463 | Les value objects de la compétence | 1 | 439 |
| 464 | `CustomSkill` — le gardien du verrou partiel | 2 | 463, 440 |
| 465 | La compétence d'espace vit en base | 3 | 441 |
| 466 | Savoir qui porte une compétence | 4 | 441 |
| 467 | Les trois use cases | 5 | 464, 465, 466, 443 |
| 468 | Le sélecteur de compétences gagne son espace | 6 | 465 |
| 469 | La teinte de catégorie devient un composant | 7 | — |
| 470 | La page de gestion et sa liste | 8 | 466, 469 |
| 471 | Le formulaire et ses trois mutations | 9 | 467, 470 |
| 472 | Les tests E2E | 10 | 463-471 |

## Ce qui commande l'ordre

**Quatre dépendances sortent vers l'épic E10** — 439, 440, 441, 443. Ce sont
elles qui donnent à `references` sa couche applicative, son `DomainError`, son
dépôt d'écriture et son infrastructure. Les deux séries partent ensemble ; ces
cartes-ci ne peuvent pas partir seules.

**469 ne dépend de rien** et peut se faire à tout moment — y compris avant tout
le reste, ce qui n'est pas une mauvaise idée : c'est la seule qui touche `players`
et le bundle CSS, donc la seule qui puisse surprendre.

**468 est isolable** : elle améliore le sélecteur de SPP indépendamment, en le
faisant entrer sous `space_scope`.

## Ce que la découpe ne suit pas

**Pas de carte d'événements.** Contrairement à la série des rosters, où la 444
monte publisher, app event et listener — « la moitié du travail de cette carte ».
La démonstration est en phase 5 : au moment où une suppression réussit, plus rien
ne cite l'uid. **Rien à nettoyer, personne à prévenir.**

**Pas de carte de migration de données.** La table naît vide.

## Une carte de l'épic E10 doit être amendée

**La 446, l'éditeur de roster.** Son sélecteur de compétences — « 146
compétences, 38 mots-clefs » — doit montrer **celles de l'espace**, donc appeler
`list_skills_for_space`.

Sans quoi on livrerait le même jour deux fonctionnalités qui s'ignorent : un
espace pourrait créer une compétence et un roster, sans pouvoir poser l'une dans
l'autre — et c'est pourtant leur emploi le plus évident ensemble.

C'est un **amendement et non une carte** : la 446 n'est pas encore écrite en
code, il n'y a rien à rattraper.

## L'épic E10 change de périmètre

Elle excluait explicitement ce chantier :

> **Les autres référentiels** — compétences, coups de pouce, star players — qui
> restent en lecture seule.

Cette phrase disparaît, et son « Terminé quand » cesse de ne parler que des
rosters et des ligues.

## Ce qui reste hors périmètre

- **Duplication d'une compétence du règlement** pour la retoucher — ce geste
  mérite sa propre décision.
- **Partage entre espaces.**
- **Coups de pouce et joueurs vedettes personnalisés** — l'épic ouvre deux
  brèches de plus, pas un back-office général.
