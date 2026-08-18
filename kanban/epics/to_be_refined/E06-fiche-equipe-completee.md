# E06 — La fiche d'équipe complétée

**État :** 3 cartes · 0 faite

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
| 48 | Onglet Trésorerie de la fiche d'équipe | `ready_to_be_done` | l'historique des mouvements financiers, lu depuis l'event store de `teams` |
| match-02 | Onglet Matchs de la fiche d'équipe | `to_be_refined` | l'historique des matchs, fragment servi par `match_report` |
| match-01 | Widget V/N/D d'une équipe | `to_be_refined` | le bilan victoires / nuls / défaites |

## Ce qui commande l'ordre

**48 est la seule prête**, et la seule autonome : les mouvements financiers
sont déjà datés et persistés dans `team_event_store`, il suffit de filtrer sur
`event_type` et de projeter en lignes de journal. Aucun BC externe.

`match-01` et `match-02` sont à raffiner, et leurs deux fiches portent une
dépendance **périmée** :

> « Dépend de : BC `match_report` (non encore créé) »

Le BC existe (`src/app/match_report/`). Le raffinage commence par là : ce qui
reste à décider, c'est le **modèle de persistance des résultats** dans ce BC —
agrégat event sourcé ou table de résultats — et si le bilan est global ou
limité à la saison courante.

Une partie de `match-01` est déjà caduque : son second point d'intégration, la
page « Mes équipes », est barré dans la carte — la page a été simplifiée et le
V/N/D n'y apparaît plus.

Les trois cartes traversent la frontière `teams` ↔ `match_report` : `teams`
héberge, `match_report` sert les fragments. C'est le patron de composition par
widgets HTMX déjà en place, pas une exception à négocier.

## Ce que l'épic ne couvre pas

- **La règle du top dog à égalité de valeur d'équipe** (`274`). Elle vit dans
  `match_report` mais ce n'est pas du code : c'est une décision de règle du jeu
  non tranchée, qu'un `>` strict a prise à la place de tout le monde. Sans épic.
- **Les autres actions sur une équipe** — modification (`45`), override admin
  d'état (`46`). Sorties de E01 après vérification, sans épic.

## Terminé quand

Aucun onglet de la fiche d'équipe n'est un libellé sans contenu, et le bilan
V/N/D de la saison courante s'affiche sur la fiche.
