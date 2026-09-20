# Les appariements de GBLR J11 après la réponse du commissaire

**Priorité : haute — une équipe apparaît deux fois sur une journée de production, une autre n'y joue pas**
**Épic :** aucune — une réparation de données, livrable d'un bloc
**Dépend de :** 551, 556
**Fichiers :**
`src/infrastructure/data_migrations/m005_gblr_j11.rs` *(nouveau)*,
`src/infrastructure/data_migrations/mod.rs`

## L'objectif

Sur la Journée 11 de GBLR Championnat, chaque équipe joue une fois, et les
deux rencontres sont celles que le commissaire a données :

| | |
|---|---|
| Lady's Ghosts (Exeloty) — Ork'Lympic Rusé (Ornox) | déjà en base, inchangé |
| Les voix des sables (merlinc) — Les loups rouges de Mideinheim (frankygno) | à rétablir |

## Ce qui l'a fait naître

Le relevé de la carte 557 sur la copie de production du 20 septembre : Ork'Lympic
Rusé y était apparié **deux fois** en J11, contre Lady's Ghosts et contre Les
loups rouges, par deux saisies à treize secondes d'intervalle le 31 août —
avant la garde de la carte 551. Les voix des sables, seule équipe enrôlée sans
match ce jour-là, était l'adversaire manquant. La donnée ne disait pas lequel
des deux appariements était le bon ; le commissaire l'a dit.

## Le changement

Une migration de données `558-gblr-j11`, sur le modèle des cartes 552 et 555.

**Recibler plutôt que supprimer et recréer.** L'appariement `01M1CKS3FX` garde
son identifiant et son brouillon de rapport ; Ork'Lympic Rusé y est remplacé
par Les voix des sables, à domicile, dans l'ordre donné par le commissaire.
Trois écritures dans une transaction : l'appariement, les colonnes domicile et
extérieur de la ligne d'affichage recopiées depuis `team_proj` — initiales
calculées par la même fonction que l'application —, et le brouillon.

**Le brouillon suit par un événement.** `SelectionUpdated` est ce qu'écrit
l'application quand un coach change la sélection d'un brouillon. La migration
l'ajoute en version 2, avec la même clé d'auteur que `MatchReportCreated`, puis
met la projection à jour. L'historique reste vrai.

**Elle ne touche que cette ligne, et seulement dans l'état attendu.**
Identifiants en dur ; vérification que l'appariement porte encore Les loups
rouges contre Ork'Lympic Rusé, que Les voix des sables est libre ce jour-là et
que le brouillon est encore `Draft`. Sinon rien n'est écrit, et un `warn` le
dit. Sur toute autre base, elle ne trouve rien et passe.

**L'autre appariement reste tel quel.** Le commissaire écrit « Ornox vs
Exeloty » ; Exeloty est à domicile en base. Le couple est le bon, et le domicile
d'un match encore à jouer se règle dans la saisie du rapport.

## Tests

| | |
|---|---|
| `le_second_adversaire_d_ornox_devient_les_voix_des_sables` | les trois écritures, sur des lignes semées avec les identifiants de production |
| `un_appariement_deja_recible_n_est_pas_touche` | rejouée sur un état qui n'est plus celui attendu, elle n'écrit rien |

## Terminé quand

Sur le calendrier de GBLR Championnat, la Journée 11 affiche Lady's Ghosts
contre Ork'Lympic Rusé et Les voix des sables contre Les loups rouges, et
chaque équipe n'y figure qu'une fois.
