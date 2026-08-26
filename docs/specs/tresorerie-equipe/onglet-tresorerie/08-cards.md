# Onglet Trésorerie · Phase 8 : cartes kanban

**Phases 1 à 7** : ce dossier

Quatre cartes, en `kanban/ready_to_be_done/`.

| N° | Carte | Dépend de |
|---|---|---|
| 434 | La fiche équipe accueille des onglets | rien |
| 435 | Lire le grand livre d'une équipe | rien |
| 436 | Le relevé de trésorerie s'affiche | 434, 435 |
| 437 | Les tests E2E de l'onglet Trésorerie | 436 |

**434 et 435 sont indépendantes** et se prennent dans n'importe quel ordre, ou
en parallèle.

## Trois choix de découpage, et leur raison

**434 ne change rien de visible, et c'est pour cela qu'elle est seule.** Elle
restructure la fiche équipe pour qu'elle sache aiguiller des onglets ; à
l'écran, avant et après sont identiques. Elle touche la page principale d'une
équipe, et une régression là-dessus doit se relire seule plutôt que mêlée à une
fonctionnalité neuve.

**435 ne touche aucun écran.** Elle livre `build_statement` — un relevé correct,
prouvé par ses tests, sans une ligne de HTML. C'est là qu'est le risque réel :
la jointure sur `event_version`, la déduplication des appels au port, le refus
sur motif inconnu. Les isoler du rendu, c'est pouvoir les corriger sans rien
démonter.

**Le port et son adapter restent dans 435.** Trois fichiers, aucune logique,
rien à démontrer isolément — ce serait une carte qu'on ouvre et referme sans
avoir rien décidé.

## Ce que cette fonctionnalité n'emporte pas

- **La carte 427** — un rapport manuel n'a pas de ligne d'affichage, donc ses
  lignes de coups de pouce n'auront ni journée ni adversaire. Le gabarit se
  replie, rien n'est bloqué, et **la 427 livrée, le relevé se complète sans
  qu'on y revienne**.
- **L'onglet « Matchs »**, qui reste inerte.
- **Les noms des coups de pouce et le nombre de spectateurs**, abandonnés en
  phase 3.

## Ce que la phase 8 clôt

Le workflow s'arrête ici. L'implémentation se fait carte par carte, sous les
règles ordinaires du `CLAUDE.md`.
