# E18 — Appariement des présents

**État :** `ready_to_be_done` — 8 cartes, 0 faite. Spécifiée par le workflow
feature les 2026-10-28 et 29.
**Conception :** `docs/specs/appariement-des-presents/`

**Pourquoi E18 et non E16** : la branche non fusionnée `sondage-presence` porte
déjà E16 et E17. Les reprendre recréerait les collisions de numéros déjà
constatées entre cette branche et `main` (cartes 507 à 512).

## La fonction

Dans une ligue amateur, on ne sait qui vient qu'au jour de la journée. Un
calendrier tiré à l'avance produit des matchs que personne ne joue.

L'organisateur ouvre une journée vide du Calendrier, décoche les équipes
absentes, voit l'aperçu d'un tirage au sort parmi les présentes — poule par
poule, en évitant les rencontres déjà jouées —, puis l'écrit au calendrier.

Le même tirage remplace celui de « Générer les rencontres », qui laisse
aujourd'hui des équipes sans match dans plus de la moitié des tirages en milieu
de saison.

## État

Aucune carte commencée.

L'algorithme de tirage est **repris** de la branche `sondage-presence`, où il a
été écrit, mesuré et testé ; le reste de la branche — sondage par e-mail,
jetons, campagne, réparation d'un tirage — ne l'est pas. Le diagnostic qui a
mené à ce choix : périmètre très supérieur au besoin, trente-trois fonctions de
plus de vingt lignes, dix-huit fichiers en conflit avec `main`.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 595 | Le tirage rejoint le domaine | `domain/draw.rs`, repris et renommé |
| 596 | MatchDay accepte ou refuse un tirage | `can_receive_draw`, `accept_draw` |
| 597 | Écrire une journée d'un bloc | `save_pairings`, atomique sous verrou |
| 598 | « Générer une journée complète » passe au nouveau tirage | le défaut de `main` corrigé |
| 599 | L'étape « Présents » | l'écran des équipes à cocher |
| 600 | L'aperçu du tirage | tirer sans écrire, retirer au sort |
| 601 | Écrire le tirage au calendrier | revérifier, écrire, annoncer |
| 602 | Les tests e2e de l'appariement des présents | le parcours dans un navigateur |

## Ce qui commande l'ordre

**598 passe avant l'écran.** Elle répare « Générer les rencontres » et se livre
seule, avant tout le reste ; elle pose aussi le chargement commun et l'émission
partagée que 599 à 601 réutilisent.

595 et 597 sont indépendantes. 599, 600 et 601 forment une chaîne dont chaque
maillon est livrable : les présents s'ouvrent avant que l'aperçu existe,
l'aperçu se voit avant que l'écriture existe.

## Ce que l'épic ne couvre pas

- **Mémoriser les présences** (R1) : la sélection ne sert qu'au tirage.
- **Demander leur présence aux coachs** — c'est le sondage par e-mail de la
  branche `sondage-presence`, laissé de côté.
- **Retoucher l'aperçu à la main** (R6) : on retire au sort, ou l'on corrige
  après écriture comme aujourd'hui.
- **L'encapsulation de `MatchDay`** : carte 594, sans épic.

## Terminé quand

Sur une journée vide, l'organisateur décoche deux absents, voit l'aperçu, écrit
au calendrier : chaque équipe présente a un match sauf l'exemptée, qui est celle
qui a le plus joué, et aucune revanche n'est programmée là où une rencontre
inédite était possible. Et « Générer une journée complète » ne laisse plus
d'équipe sans match hors l'exemptée.
