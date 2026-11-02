# La suite e2e ralentit avec sa base

**Priorité : haute** — chaque vérification complète coûte désormais une relance,
et un échec réel se noie dans les dépassements de délai
**Trouvée par :** la fusion de la carte 634 (2026-11-02).

## Le symptôme

La suite complète prenait **9 minutes** le 2026-11-01 ; elle en prend **20 à
22** le lendemain, sans changement de périmètre notable. Sur la carte 634, deux
exécutions de suite ont donné deux ensembles d'échecs **différents** :

| exécution | échecs | erreurs | nature |
|---|---|---|---|
| 1 | 11 | 53 | surtout des fixtures |
| 2 | 1 | 11 | `Timeout 30000ms exceeded` — l'assistant de compétition, la page des matchs d'une équipe |

Relancé seul, **chaque fichier en cause passe**. Aucune des pages en délai ne
passe par le code de la 634.

## Le soupçon

La base `kreek_e2e` ne fait que grossir : chaque exécution crée ses
compétitions (`build_full_competition`), ses équipes, ses comptes rendus, et
rien ne les efface. Au 2026-11-02 : 124 Mo, 2387 équipes, 1466 comptes rendus.
Les pages qui listent par espace — calendrier, équipes, assistant — lisent tout
ce qui s'est accumulé dans l'espace de test, qui est **un seul espace** pour
toute la suite.

Le soupçon reste à **mesurer** avant de corriger : durée de la suite et temps
des requêtes lentes, sur une base fraîche (`make init_e2e_db`) puis sur la base
actuelle.

## Ce qu'il faut faire

1. **Mesurer** : la suite complète sur une base fraîche, puis sur la base
   actuelle. Si l'écart ne vient pas de là, la carte change d'objet — le dire
   plutôt que de corriger à côté.
2. **Si la base est en cause**, repartir d'une base propre à chaque exécution
   complète. Deux voies, à trancher à la lecture du code :
   - `make e2e` remet la base à zéro avant de lancer pytest — mais le serveur
     e2e tourne déjà, et une remise à zéro sous un serveur vivant doit être
     éprouvée (pool de connexions, migrations au démarrage) ;
   - ou un nettoyage par la suite elle-même, en tête de session pytest.
3. **La CI n'est pas concernée** : sa base est un service Postgres neuf à chaque
   job. La correction ne doit rien y changer.

## Hors du périmètre

Les requêtes lentes que la mesure révélerait **en production** : une base de
ligue n'accumule pas comme la base de test, mais si une page s'effondre à 2000
équipes, c'est une carte à part, avec ses chiffres.

## Terminé quand

- [ ] la suite complète retrouve une durée mesurée et stable, sur deux
      exécutions de suite ;
- [ ] deux exécutions de suite donnent le même résultat.

## Conséquences sur les données de production

Aucune : seule la base e2e locale est touchée.
