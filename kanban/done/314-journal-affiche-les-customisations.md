# Le journal des évolutions affiche les customisations

**Priorité : haute**
**Dépend de :** `308-players-customisation-endpoints.md`
**Bloque :** `309-players-customisation-e2e.md` (scénario 2)
**Contexte :** `players` — widget journal des évolutions

## Le manque

`evolution_journal_widget::evolution_log_row` ne connaît que trois
événements — `InitialSkillEarned`, `PlayerSkillPurchased`,
`PlayerStatIncreased`. **Les quatre événements de customisation tombent dans
`_ => None`** : rien de ce qu'un commissaire applique n'apparaît au journal.

Une compétence offerte, une caractéristique posée, un prix ajusté, des SPP
crédités — le joueur change, et son journal reste muet. C'est exactement ce que
la phase 1 voulait éviter en exigeant qu'une customisation soit tracée avec le
nom de son auteur.

## Le détail qui trahit

Le libellé `AcquisitionMode::Customised => "Customisation"` **existe déjà** dans
ce fichier. Il est inatteignable : le journal lit les événements bruts, pas
l'agrégat replié, et aucun événement rendu ne porte ce mode.

L'affichage a été préparé, la source jamais branchée. Le repli `_ => None` a
absorbé le manque sans que rien ne le signale — c'est le prix du joker, déjà
payé une fois par `to_app_event` (carte 313).

## Ce qu'il faut afficher

Quatre bras, un par famille. Chacun porte **le nom du commissaire**, que
l'événement transporte déjà dans son champ `author` — c'est là toute la raison
d'être de la traçabilité posée en phase 1.

| Événement | Libellé | Coût | Valeur |
|---|---|---|---|
| `PlayerSkillCustomised` | le nom de la compétence | — | — |
| `PlayerStatCustomised` | « Caractéristique : Agilité » | — | — |
| `PlayerValueCustomised` | « Prix ajusté » | — | le delta signé, en kPo |
| `PlayerSppCustomised` | « SPP crédités » | le montant | — |

Les colonnes **coût** et **valeur** restent vides là où la notion n'existe
pas : une compétence customisée ne coûte aucun SPP et n'ajoute aucune valeur —
c'est la règle de la phase 3, et un `0` affiché la contredirait en laissant
croire à un calcul.

La colonne **origine** dit « Customisation », et le mode porte la pastille
`🛠️` de la maquette.

---

## Checklist

- [x] Les quatre bras dans `evolution_log_row`
- [x] Le nom de l'auteur affiché
- [x] Colonnes coût et valeur vides quand la notion n'existe pas
- [x] Test : chaque famille produit sa ligne, avec son auteur
- [x] Test : un `0` n'est jamais affiché là où la notion n'existe pas

## Point de vigilance

Le joker `_ => None` de `evolution_log_row` restera après cette carte — le
journal ignore légitimement `PlayerCreated`, `PlayerRenamed` et les impacts de
match. Il continuera donc d'absorber en silence tout événement futur qu'on
oublierait d'y ajouter. Le remplacer par une énumération exhaustive rendrait le
compilateur bavard à chaque nouvel événement ; c'est un arbitrage à faire, pas
un oubli à corriger au passage.

## Réalisé, et deux écarts au passage

**`origin` devient `String`.** Il était `&'static str` ; l'origine nomme
désormais le commissaire, ce qu'aucune chaîne constante ne peut faire. Les
trois lignes existantes gagnent un `.to_string()`.

**La pastille de mode gagne sa classe** : `mode_css` sur le VM, au lieu du
`mode-chip-chosen` codé en dur dans le template. Une customisation ne doit pas
se confondre visuellement avec une progression normale. `.mode-chip-custom` a
migré du CSS du widget de customisation vers celui de la page — le journal ne
charge pas le premier, la pastille y serait restée sans style.

**Un défaut évité de justesse** : la première rédaction formatait le signe du
prix via un `i8`. `KpoDelta` est un `i32` : un ajustement de −300 se serait
affiché « +212 ». Un test le fixe désormais.
