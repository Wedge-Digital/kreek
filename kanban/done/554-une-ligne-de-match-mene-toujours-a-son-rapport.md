# Une ligne de match mène toujours à son rapport

**Priorité : moyenne — signalée à la livraison des cartes 551 à 553**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/competitions/io/web/resultats_view.rs`,
`src/app/competitions/io/web/resultats_tab_controller.rs`,
`src/app/competitions/io/web/widgets/team_matches_widget.rs`

## L'objectif

Cliquer sur une ligne de l'onglet **Matchs** d'une équipe mène à la saisie du
rapport, comme dans l'onglet Calendrier.

## Ce qui l'a fait naître

Les deux écrans construisaient le même lien de deux façons :

| écran | source du lien |
|---|---|
| onglet Calendrier | `from_pairing(pairing_id)`, construit à la volée |
| onglet Matchs d'une équipe | `match_report_url`, lu dans la projection |

Or `competition_match_display_proj.match_report_url` **n'est posé qu'à la
confirmation du rapport**. Un match à venir n'en a donc aucune, et sa ligne
n'était pas cliquable.

**L'onglet Résultats ne montrait pas le défaut** : il ne liste que
`in_progress` et `completed`, qui ont tous leur adresse. L'onglet Matchs est le
seul écran à afficher les trois statuts — c'est là, et là seulement, que le trou
se voyait.

Le gabarit, lui, n'était pas en cause : `match-widget.html` sait rendre le lien
et la classe `match-widget--clickable` dès que `report_url` est présent. C'est le
view model qui ne lui donnait rien.

## Le changement

Une fonction `lien_du_rapport` : l'adresse directe quand la projection la porte
— elle évite une redirection —, **sinon `from_pairing`**, qui résout le rapport
de l'appariement à la volée.

Les deux écrans font désormais la même chose d'une seule façon. Le calendrier
n'avait pas le défaut parce qu'il faisait déjà ce repli sans jamais lire
`match_report_url` ; c'est sa version qui l'emporte.

`space_id` descend jusqu'au view model, `from_pairing` en ayant besoin. C'est le
seul élargissement de signature de la carte.

## L'effet de bord, assumé

Sur l'onglet Résultats, un match dont la projection aurait perdu son
`match_report_url` devient cliquable par le repli, là où il ne l'était pas.

C'est souhaitable : `from_pairing` résout le rapport au moment du clic, quand
`match_report_url` est une adresse figée qui peut être périmée — un rapport
dépublié puis republié, par exemple. Le repli est plus juste que ce qu'il
remplace.

## Ce que la carte ne touche pas

L'autorisation. `ResultAuthorization::allows` décide seule si la ligne est
cliquable, et elle prime sur le lien : sans droit, aucune adresse n'est
construite, pas même le repli. Un test le verrouille — sans lui, le repli
donnerait une adresse à qui n'a pas le droit de la suivre.

## Tests

Trois tests unitaires, un par cas :

| | |
|---|---|
| `un_match_a_venir_mene_quand_meme_a_son_rapport` | **le cas qui manquait** — sans `match_report_url`, l'adresse vise l'appariement |
| `l_adresse_directe_est_preferee_au_repli` | le repli évite une redirection, il ne la provoque pas |
| `sans_autorisation_aucune_adresse_meme_a_venir` | le droit prime sur les deux |

## Terminé quand

Dans l'onglet Matchs d'une équipe, la ligne du prochain match est cliquable et
mène à la saisie de son rapport — comme dans l'onglet Calendrier.
