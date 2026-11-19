# E06 — La fiche d'équipe complétée

**État :** 8 cartes · 0 faite. Deux cartes d'intention ont été **remplacées**
par des cartes conçues : la 48 par les quatre de la trésorerie (434 à 437,
`docs/specs/tresorerie-equipe/`), et `match-02` par les trois de l'onglet Matchs
(476 à 478, `docs/specs/matchs-d-une-equipe/`).

## La fonction

La fiche d'équipe affiche des onglets qui ne font rien. Deux libellés y sont
posés sans aucun comportement derrière :

```html
teams-team-detail.html:148    <div class="tab">Matchs</div>       ← inerte
teams-team-detail.html:149    <div class="tab">Trésorerie</div>   ← inerte
```

Pas de `hx-get`, pas de handler, pas de fragment. L'utilisateur clique et rien
ne se passe — ce qui est pire qu'une absence d'onglet : la fiche promet une
information qu'elle ne sait pas donner.

L'épic remplit ces deux onglets et ajoute le bilan V/N/D, la troisième zone
manquante du même écran.

## Les cartes

| # | Intitulé | Dossier | Apport |
|---|---|---|---|
| **434** | La fiche équipe accueille des onglets | `ready_to_be_done` | le mécanisme d'aiguillage, sans changement visible |
| **435** | Lire le grand livre d'une équipe | `ready_to_be_done` | le port et l'adapter sur `teams__treasury_ledger` |
| **436** | Le relevé de trésorerie s'affiche | `ready_to_be_done` | l'onglet, ses lignes, son solde courant |
| **437** | Les tests e2e de l'onglet Trésorerie | `ready_to_be_done` | le relevé sous Playwright |
| **476** | Le bloc de match devient un composant | `ready_to_be_done` | gabarit et feuille extraits, aucun écran ne change |
| **477** | L'onglet Matchs d'une équipe | `ready_to_be_done` | la liste, servie par `competitions` |
| **478** | Les tests e2e de l'onglet Matchs | `ready_to_be_done` | dont la non-régression de la page compétition |
| match-01 | Widget V/N/D d'une équipe | `to_be_refined` | le bilan victoires / nuls / défaites |
| ~~48~~ | ~~Onglet Trésorerie~~ | `cancelled` | **remplacée** par 434 à 437 |
| ~~match-02~~ | ~~Onglet Matchs~~ | `cancelled` | **remplacée** par 476 à 478 |

## Ce qui commande l'ordre

**Les quatre cartes de la trésorerie sont prêtes**, et le chantier est
autonome : aucun BC externe. La 434 vient d'abord — elle pose l'aiguillage
d'onglets sans rien changer à l'écran — puis 435, 436, 437.

La 48 croyait qu'il fallait projeter l'event store. **C'est faux** : le grand
livre `teams__treasury_ledger` existe et est écrit depuis l'origine, il n'a
simplement jamais été lu. Le chantier est donc une lecture, pas une projection
— et c'est ce genre d'écart que le workflow feature sert à trouver.

`match-01` reste à raffiner, et sa fiche porte une dépendance **périmée** :

> « Dépend de : BC `match_report` (non encore créé) »

Le BC existe. Et la question qu'elle posait — le **modèle de persistance des
résultats** — est **sans objet** : la conception de l'onglet Matchs l'a
établi, `competition_match_display_proj` porte déjà tout, et le bilan V/N/D
n'est que cette même requête, comptée. Ce qui reste à décider pour `match-01`
tient en une ligne : le bilan est-il global ou limité à la saison courante ?

Une partie de `match-01` est déjà caduque : son second point d'intégration, la
page « Mes équipes », est barré dans la carte — la page a été simplifiée et le
V/N/D n'y apparaît plus.

Les cartes traversent la frontière `teams` ↔ **`competitions`**, et non
`teams` ↔ `match_report` comme l'épic le croyait : c'est `competitions` qui
possède `competition_match_display_proj`, donc c'est lui qui sert les
fragments. `teams` héberge et compose — le patron déjà en place à la ligne 153
de `teams-team-detail.html`, où la fiche charge le widget des joueurs.

**Les 476 à 478 dépendent de la 434**, qui pose le mécanisme d'onglets : celui
des Matchs est aujourd'hui un `<div class="tab">` sans `hx-get` ni handler, il
n'a nulle part où se brancher avant elle.

## Ce que l'épic ne couvre pas

- **La règle du top dog à égalité de valeur d'équipe** (`274`). Elle vit dans
  `match_report` mais ce n'est pas du code : c'est une décision de règle du jeu
  non tranchée, qu'un `>` strict a prise à la place de tout le monde. Sans épic.
- **Les autres actions sur une équipe** — modification (`45`), override admin
  d'état (`46`). Sorties de E01 après vérification, sans épic.

## Terminé quand

Aucun onglet de la fiche d'équipe n'est un libellé sans contenu, et le bilan
V/N/D de la saison courante s'affiche sur la fiche.
