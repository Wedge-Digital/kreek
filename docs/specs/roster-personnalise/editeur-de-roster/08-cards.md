# Éditeur de roster · Phase 8 : cartes kanban

**Phases 1 à 7** : ce dossier · **Épic :** E10 — Référentiels éditables

Neuf cartes, en `kanban/ready_to_be_done/`.

## Vague 1 — le socle, sans écran

| N° | Carte | Dépend de |
|---|---|---|
| 439 | Les value objects du roster | rien |
| 440 | `CustomRoster`, le gardien | 439 |
| 441 | Le roster d'espace vit en base | rien |
| 442 | Savoir quelles équipes jouent un roster | rien |

**439, 441 et 442 sont indépendantes** et se prennent dans n'importe quel ordre.

## Vague 2 — l'orchestration

| N° | Carte | Dépend de |
|---|---|---|
| 443 | Les trois use cases | 440, 441, 442 |
| 444 | La chaîne d'événements du roster supprimé | 443 |

## Vague 3 — les écrans

| N° | Carte | Dépend de |
|---|---|---|
| 445 | La liste des rosters d'un espace | 441, 442 |
| 446 | L'éditeur de roster | 443, 445 |
| 447 | Les tests E2E | 446 |

## Quatre choix de découpage, et leur raison

**439 est seule parce qu'elle touche un autre BC.** Le déplacement des cinq
value objects vers `shared_kernel` modifie `team_creation` ; une régression
là-dessus doit se relire sans être mêlée à un domaine neuf.

**441 porte le changement de signature de `find_team_by_uid`** — huit sites
d'appel dans cinq BCs. Mécanique, mais ça touche partout, et l'isoler donne un
commit qui se relit d'un coup d'œil.

**442 est utile hors de cette épic.** Aucune requête ne sait aujourd'hui
répondre à « quelles équipes jouent ce roster » ; la colonne servira à
quiconque posera la question.

**445 avant 446, et c'est contre-intuitif.** La liste est petite, l'éditeur est
le morceau. Mais la liste porte le résolveur `ISpaceOwnership` pour
`roster_uid`, qui rend le contrôle d'accès **structurel** — un roster d'un autre
espace rend `404` avant le handler, et l'éditeur en hérite gratuitement.
L'inverse obligerait l'éditeur à contrôler à la main, puis à défaire.

## Ce que l'épic n'emporte pas

- **La carte 438** — le `filter_map` muet de `builders.rs`. La suppression d'un
  roster ouvre une fenêtre asynchrone pendant laquelle un tier cite un uid mort ;
  la 438 la rend audible. Elle existe indépendamment et **gagne à passer avant**.
- **Aucun import depuis un roster existant** : dupliquer les Elfes Sylvestres
  pour les retoucher est le geste le plus probable d'un ligueur, et c'est
  précisément pour ça qu'il mérite sa propre décision.
- **Aucun partage entre espaces**, aucune traduction.

## Ce que la phase 8 clôt

Le workflow s'arrête ici. L'implémentation se fait carte par carte, sous les
règles ordinaires du `CLAUDE.md`.
