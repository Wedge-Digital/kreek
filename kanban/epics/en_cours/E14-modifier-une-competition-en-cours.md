# E14 — Modifier une compétition en cours

**État :** `en_cours` — 10 cartes, 5 faites (417 à 421, le 2026-08-28).
Les vagues 1 — le socle — et 2 — la place — sont complètes ; la vague 3 est
entamée par le panneau qui pose la forme des quatre autres, puis par celui qui
porte le cas commandant l'épic — modifier le barème recalcule le classement
publié. Restent 423 à 425 et les tests e2e de l'onglet. Spécifiée par
le workflow feature les 2026-08-25 et 26.
**Conception :** `docs/specs/modifier-une-competition/`

## La fonction

Une compétition se règle aujourd'hui **une fois, à sa création**, par un
magicien en cinq étapes. Après quoi rien ne se corrige : un barème mal saisi, un
nom fautif, une poule de trop restent tels quels pour la saison entière.

Cette épic ajoute un onglet **Paramètres** à l'administration de compétition,
qui rouvre cinq de ces réglages sur une saison en cours — informations
générales, barème de classement, poules, coups de pouce par tier, visibilité.

Le cas qui commande tout le reste : **modifier le barème recalcule le classement
publié**, immédiatement, dans le même POST. Sans ce recalcul, changer le barème
en cours de saison produirait un classement qui mélange deux règles.

## État

**Cinq cartes sur dix sont faites, le 2026-08-28.** L'onglet existe : `417` a
posé la garde de statut sur les quatre use cases d'écriture, `418` a rendu le
classement rejouable, `419` a sorti le tableau de bord et les résultats de
l'administration, `420` a créé la coquille de l'onglet, et `421` son premier
panneau — celui qui pose la forme des quatre autres.

Restent `422` à `425` — les quatre panneaux — et `426`, les tests e2e.

Ce qui existait déjà et a été réemployé : les quatre use cases d'écriture du
magicien (`save_competition_rules`, `save_competition_structure`,
`save_competition_invitations`, `update_draft_competition`), aucun n'ayant de
garde de statut ; le widget de sélection des coups de pouce du BC `references` ;
et `ranking_lines`, dont la forme cumulative permet le rejeu sans relire un seul
rapport de match.

## Les cartes

| N° | Carte | Vague |
|---|---|---|
| 417 | Les réglages deviennent gardés par le domaine | 1 — socle |
| 418 | `ranking` sait se rejouer | 1 — socle |
| 419 | Le tableau de bord et les résultats quittent l'administration | 2 — la place |
| 420 | L'onglet Paramètres, coquille vide | 2 — la place |
| 421 | Panneau « Informations générales » | 3 — panneaux |
| 422 | Panneau « Points de classement » | 3 — panneaux |
| 423 | Panneau « Poules » | 3 — panneaux |
| 424 | Panneau « Tiers & coups de pouce » | 3 — panneaux |
| 425 | Panneau « Visibilité » | 3 — panneaux |
| 426 | Les tests E2E de l'onglet | 4 |

## Ce qui commande l'ordre

**417 avant 423 et 424** — ces deux panneaux appellent les méthodes domaine
qu'elle pose.

**418 avant 422** — le panneau du barème déclenche le rejeu, qui doit exister.

**419 avant 420** — retirer le tableau de bord fait du Résumé l'onglet par
défaut ; l'onglet Paramètres se pose ensuite sur un aiguillage propre.

**420 avant les cinq panneaux** — ils remplissent ses conteneurs.

Le reste est du confort : 421, 422 et 425 se livrent dans n'importe quel ordre.

## Ce que l'épic ne couvre pas

- **Aucune suppression ni réinitialisation de saison** — la zone de danger de la
  maquette a été retirée. La direction est l'archivage (carte 414).
- **Aucune modification de roster, de budget ni d'XP de départ** : affichés en
  libellé-valeur, hors sujet dans cette version.
- **Aucun ajout ni retrait de tier.**
- **Aucune gestion des administrateurs** : le panneau est un affichage.
- **Ni la carte 415** (le plafond de participants) **ni la 416** (les treize
  routes sans contrôle d'accès) : deux défauts trouvés en instruisant cette
  épic, indépendants d'elle.

## Terminé quand

Un administrateur porte la victoire de 2 à 3 points sur une saison en cours, et
**le classement affiché aux coachs montre les nouveaux totaux avant qu'il ait
quitté la page**.

C'est le seul critère qui prouve la chaîne entière — l'écran, le use case, le
port de commande, le rejeu dans l'autre BC, et la transaction qui remplace les
lignes.
