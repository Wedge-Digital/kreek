# Épics

Une **épic** est une grande fonction qui regroupe plusieurs cartes. Elle existe
pour une seule raison : garder une vue de haut niveau sur les chantiers, que la
liste plate des cartes ne donne pas.

L'épic dit **pourquoi** et **quand c'est fini**. Les cartes disent **comment**.

## Cycle de vie

```
to_be_refined → ready_to_be_done → en_cours → done
```

| Dossier | Contenu |
|---|---|
| `to_be_refined/` | Épic dont une carte au moins reste à raffiner |
| `ready_to_be_done/` | Toutes les cartes sont prêtes à implémenter |
| `en_cours/` | Au moins une carte est commencée, l'épic n'est pas close |
| `done/` | Le critère « Terminé quand » est vérifié |

L'état d'une épic suit **la plus en amont** de ses cartes : une épic dont neuf
cartes sont prêtes et une à raffiner reste en `to_be_refined/`.

## Conventions

- **Un fichier par épic**, nommé `E<NN>-<slug>.md`. Le préfixe `E` évite toute
  collision avec la numérotation des cartes.
- **Une carte appartient à une seule épic, ou à aucune.** Les cartes sans épic
  sont listées ci-dessous — on ne les range pas dans une épic « Divers », qui
  simulerait une vue de haut niveau au lieu d'en donner une.
- Chaque épic porte les sections : *La fonction* · *État* · *Les cartes* ·
  *Ce qui commande l'ordre* · *Ce que l'épic ne couvre pas* · *Terminé quand*.
- **« Terminé quand » est un critère observable**, jamais « toutes les cartes
  sont dans `done/` ». Une épic close se constate à l'écran ou à la mesure.

## Les épics

| Épic | État | Cartes |
|---|---|---|
| [E01 — Saison de jeu : le cycle de vie d'une équipe](done/E01-saison-de-jeu.md) | `done` | 10 |
| [E02 — Notifications e-mail de compétition](ready_to_be_done/E02-notifications-email.md) | `ready` | 11 |
| [E03 — Front : ni saut, ni clignotement](ready_to_be_done/E03-front-ni-saut-ni-clignotement.md) | `ready` | 5 |
| [E04 — Les verrous architecturaux](ready_to_be_done/E04-verrous-architecturaux.md) | `ready` | 6 |
| [E05 — Couverture e2e du déjà livré](ready_to_be_done/E05-couverture-e2e.md) | `ready` | 4 |
| [E06 — La fiche d'équipe complétée](to_be_refined/E06-fiche-equipe-completee.md) | `to_be_refined` | 3 |
| [E07 — Entrées utilisateur et identité](ready_to_be_done/E07-entrees-utilisateur-et-identite.md) | `ready` | 2 |
| [E08 — Mode customisation : finir la livraison](ready_to_be_done/E08-mode-customisation.md) | `ready` | 2 |
| [E09 — BC `news`](to_be_refined/E09-bc-news.md) | `to_be_refined` | 2 |
| [E10 — Référentiels éditables](to_be_refined/E10-referentiels-editables.md) | `to_be_refined` | 2 |
| [E11 — Savoir ce qui se passe en production](done/E11-journal-de-production.md) | `done` | 9 |

## Les cartes sans épic

Elles ne relèvent d'aucune grande fonction. Les lister ici est le seul moyen
qu'elles ne disparaissent pas de la vue d'ensemble.

**Décisions de règle du jeu en attente** — ni l'une ni l'autre n'est un travail
de code tant que la règle n'est pas tranchée :

| Carte | Question ouverte |
|---|---|
| `274-inducements-egalite-de-tv` | À valeurs d'équipe égales, qui est top dog ? Un `>` strict désigne le domicile, personne ne l'a décidé |
| `TBD-62-anti-optimisateur` | Contenu à définir |

**Actions sur une équipe, hors cycle de saison** — sorties de E01 après
vérification : rien dans le code ne les implémente.

| Carte | État |
|---|---|
| `45-team-modification` | à raffiner |
| `46-team-customisation-admin` | à raffiner |

**Dette technique diffuse** — sept cartes courtes et indépendantes, chacune une
session brève : `06` (groupement O(n²)), `09` (`Entity::eq` shadowe
`PartialEq`), `10` (`bypass_auth` dans `AppState`), `11` (`initials()`
dupliquée), `14` (`cloudinary_transform()` privée), `15` (URLs par
`String::replace`), `254` (message d'erreur JSON trompeur).

**Isolées** :

| Carte | Note |
|---|---|
| `13-mock-data-competition-detail` | Les onglets Équipes et Stats servent de la donnée fictive **en production**. C'est la seule dette visible par un utilisateur |
| `60-jersey-numbers-at-submission` | Attribution des numéros de maillot à la soumission |
| `357-le-champ-tags-est-en-ecriture-seule` | Quatre formes écrites, aucun lecteur : `find_by_tag()` n'a pas d'appelant. À trancher — compléter l'abstraction ou la supprimer |
| `352-match-report-confirmed-passe-par-le-publisher` | Deux use cases émettent un app event directement, ce que `CLAUDE.md` interdit. Trouvée par la carte 350. Débloque un axe « pas d'`app_event_bus` dans `use_cases/` » |
| `61-rendre-accessible-un-espace-publiquement` | **Fichier vide, sans extension `.md`** — à écrire ou à supprimer |
| `TBD-63-implémenter-les-notifications-email` | **Vraisemblablement caduque** : remplacée par les dix cartes de E02. À vérifier puis déplacer en `cancelled/` |
