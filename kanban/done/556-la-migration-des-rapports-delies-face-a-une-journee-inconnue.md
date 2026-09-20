# La migration des rapports déliés face à une journée inconnue

**Priorité : haute — la migration refuse le démarrage sur une donnée qu'elle ne prévoit pas**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** 552
**Fichiers :**
`src/infrastructure/data_migrations/m003_rapports_delies.rs`

## L'objectif

`552-rapports-delies` traite tous les rapports déliés qu'elle trouve, quelle
que soit la journée qu'ils portent, et dit lequel la bloque si elle échoue.

## Ce qui l'a fait naître

Le 20 septembre, le serveur de développement a refusé de démarrer :

```
migration de données en échec : création de l'appariement : error returned
from database: insert or update on table "competition_match_day_pairings"
violates foreign key constraint "competition_match_day_pairings_match_day_id_fkey"
```

Huit rapports `ReadyToPublish` portaient `round_id = 01ARZ3NDEKTSV4RRFFQ69G5FAV`
— l'ULID d'exemple de la spécification, sur une journée qui n'a jamais existé.
Ce sont les résidus du test e2e TC-07 de `test_match_report_recap.py`, qui
créait volontairement un brouillon sur un « round_id syntaxiquement valide mais
inconnu » pour éprouver la dégradation gracieuse. Le test a été retiré le
17 septembre (commit `e15a65d7`) ; ses rapports des 6 et 10 septembre, non.

La migration ne connaît qu'un cas non liable : la journée occupée. Une journée
absente n'a aucun appariement, donc `journee_occupee` répond non, et
`creer_appariement` insère un `match_day_id` orphelin. La clé étrangère refuse.

**Le message ne nommait pas le coupable.** Il reprend le texte de Postgres, sans
`match_report_id` ni `round_id` : il a fallu une requête en base pour retrouver
les huit lignes.

## Ce que ça ferait en production

Le processus meurt avant d'ouvrir le port, et le redémarrage automatique rejoue
le même échec en boucle : indisponibilité totale jusqu'à intervention manuelle.
La base, elle, reste intacte — la transaction est annulée entière, la migration
n'est pas marquée.

Relevé sur la copie de production du 20 septembre : **37 rapports déliés
vivants, tous sur une journée existante**. Le cas ne s'y présente pas
aujourd'hui, mais une journée supprimée entre le dump et le déploiement
suffirait. Une migration qui dépend de la donnée du jour n'est pas sûre.

## Le changement

**Un second cas non liable.** Un rapport dont la journée n'existe pas ne peut
pas avoir d'appariement, comme un rapport dont les équipes jouent déjà ce
jour-là. Il suit le même chemin : annulé, ses deux équipes libérées.

**Des erreurs qui nomment le rapport.** `creer_appariement` et
`inserer_ligne_affichage` portent `match_report_id` et `round_id` dans leur
message. Ce qui bloque le démarrage doit se lire dans la ligne qui le dit.

**Un journal qui ne prétend rien avant le commit.** `lier` écrivait « rapport
délié rattaché à son appariement » à chaque rapport, en `info`. Quand la
transaction est annulée ensuite, ces lignes restent dans le journal et décrivent
des écritures qui n'ont jamais eu lieu. Elles passent en `debug`, et c'est le
registre qui, au commit, annonce le nombre d'agrégats touchés.

## Tests

Le module n'en avait aucun. Un test `#[sqlx::test]`, sur le modèle de `m001` :

| | |
|---|---|
| `un_rapport_sur_une_journee_inconnue_est_annule_et_libere_ses_equipes` | **le cas qui manquait** — la migration passe, le rapport est `Cancelled`, ses deux équipes reviennent en `ReadyToPlay` |

## Terminé quand

Le serveur démarre sur une base qui porte un rapport délié sur une journée
inconnue, et le journal dit ce que la migration en a fait.
