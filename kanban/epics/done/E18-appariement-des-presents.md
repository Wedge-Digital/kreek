# E18 — Appariement des présents

**État :** close le 2026-11-01 — 8 cartes, 8 faites. Spécifiée par le workflow
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

6/8 : le tirage vit dans `domain/draw.rs` (612), `MatchDay` le vérifie (613), `save_pairings` écrit une journée d'un bloc (614), « Générer une journée complète » s'en sert (615), l'étape « Présents » s'ouvre sur une journée vide (616), l'aperçu tire parmi les présentes sans rien écrire (617), l'écriture revérifie tout avant d'écrire la journée (618), et les tests e2e tiennent le parcours entier dans un navigateur (619). Le critère de clôture est constaté par `tests/e2e/test_schedule_draw.py` : décoche des absentes, aperçu, écriture, une rencontre par présente hors l'exemptée, l'exemptée parmi les plus servies, et « Générer une journée complète » ; l'évitement des revanches l'est par les tests unitaires du domaine (612).

L'algorithme de tirage est **repris** de la branche `sondage-presence`, où il a
été écrit, mesuré et testé ; le reste de la branche — sondage par e-mail,
jetons, campagne, réparation d'un tirage — ne l'est pas. Le diagnostic qui a
mené à ce choix : périmètre très supérieur au besoin, trente-trois fonctions de
plus de vingt lignes, dix-huit fichiers en conflit avec `main`.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 612 | Le tirage rejoint le domaine | `domain/draw.rs`, repris et renommé |
| 613 | MatchDay accepte ou refuse un tirage | `can_receive_draw`, `accept_draw` |
| 614 | Écrire une journée d'un bloc | `save_pairings`, atomique sous verrou |
| 615 | « Générer une journée complète » passe au nouveau tirage | le défaut de `main` corrigé |
| 616 | L'étape « Présents » | l'écran des équipes à cocher |
| 617 | L'aperçu du tirage | tirer sans écrire, retirer au sort |
| 618 | Écrire le tirage au calendrier | revérifier, écrire, annoncer |
| 619 | Les tests e2e de l'appariement des présents | le parcours dans un navigateur |

## Ce qui commande l'ordre

**615 passe avant l'écran.** Elle répare « Générer les rencontres » et se livre
seule, avant tout le reste ; elle pose aussi le chargement commun et l'émission
partagée que 616 à 618 réutilisent.

612 et 614 sont indépendantes. 616, 617 et 618 forment une chaîne dont chaque
maillon est livrable : les présents s'ouvrent avant que l'aperçu existe,
l'aperçu se voit avant que l'écriture existe.

## Ce que l'épic ne couvre pas

- **Mémoriser les présences** (R1) : la sélection ne sert qu'au tirage.
- **Demander leur présence aux coachs** — c'est le sondage par e-mail de la
  branche `sondage-presence`, laissé de côté.
- **Retoucher l'aperçu à la main** (R6) : on retire au sort, ou l'on corrige
  après écriture comme aujourd'hui.
- **L'encapsulation de `MatchDay`** : carte 611, sans épic.

## Terminé quand

Sur une journée vide, l'organisateur décoche deux absents, voit l'aperçu, écrit
au calendrier : chaque équipe présente a un match sauf l'exemptée, qui est celle
qui a le plus joué, et aucune revanche n'est programmée là où une rencontre
inédite était possible. Et « Générer une journée complète » ne laisse plus
d'équipe sans match hors l'exemptée.
