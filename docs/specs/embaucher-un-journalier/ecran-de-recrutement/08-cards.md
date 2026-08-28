# Écran de recrutement · Phase 8 : cartes kanban

**Phases 1 à 7** : ce dossier · **Conception** : `../00-conception.md`

Six cartes, en `kanban/ready_to_be_done/`.

| N° | Carte | Dépend de |
|---|---|---|
| 454 | Un journalier est un joueur | rien |
| 455 | Le journalier naît avec le rapport | 454 |
| 456 | Le journalier disparaît | 455 |
| 457 | Le panier accueille un journalier | 454 |
| 458 | L'écran affiche les journaliers recrutables | 456, 457 |
| 459 | Les tests E2E | 458 |

**455 et 457 se prennent en parallèle** une fois la 454 passée.

## Quatre choix de découpage

**454 est seule, et c'est la carte la plus risquée de la série.** Elle change
quatre requêtes SQL qu'aucun compilateur ne vérifie, sur une projection que
quatre écrans lisent. Une erreur là-dessus se voit partout et se diagnostique
mal : un journalier invisible ne produit aucune erreur, seulement un nombre faux
au rapport suivant. Elle porte aussi le commentaire de `journeymen_value`, qui
protège la valeur d'équipe de toutes les équipes hors match.

**455 avant 456 — naître avant de mourir.** Livrée seule, la 455 produit des
journaliers qui s'accumulent : c'est ce qui impose l'ordre, et ce qui interdit
de livrer l'une sans l'autre.

**456 est séparée parce qu'elle comble un manque du projet.** `TeamsAppEvent` ne
publiait aucun changement de phase ; `RecruitmentPhaseValidated` en app event
servira à tout BC qui voudra réagir à la fin d'un recrutement.

**458 dépend de 456 autant que de 457.** Sans la disparition, l'écran
afficherait des journaliers de matchs anciens — un panneau qui grossit et ne se
vide jamais.

## Ce que l'ensemble n'emporte pas

- **Aucune table neuve, aucune feuille CSS neuve.**
- **Aucun changement au déroulé du match** : le rapport garde ses `TempPlayer`.
- **Aucune reprise de l'existant** : les rapports en cours à la livraison
  n'auront pas de journaliers dans `players`, et leur phase de recrutement n'en
  proposera aucun — dégradé mais correct.
- **La fenêtre asynchrone n'est pas traitée** : deux écrans séparent la
  naissance de l'affichage, et un rafraîchissement rattraperait le cas limite.

## Ce que la phase 8 clôt

Le workflow s'arrête ici. L'implémentation se fait carte par carte, sous les
règles ordinaires du `CLAUDE.md`.
