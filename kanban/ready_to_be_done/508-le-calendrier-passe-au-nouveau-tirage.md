# Le Calendrier passe au nouveau tirage

**Priorité : haute — la correction ne sert à rien tant que personne ne l'appelle**
**Épic :** E16 — Sondage de présence
**Dépend de :** 507, qui écrit `tirer`
**Fichiers :** `src/app/competitions/use_cases/admin/generate_pairings.rs`,
`src/app/competitions/use_cases/admin/generate_all_pairings.rs`,
`src/app/competitions/domain/match_day.rs`

## L'objectif

`generate_pairings` appelle `tirer` au lieu de `generate_round_pairings`, et la
seconde disparaît.

Le bénéfice est immédiat et n'attend pas le sondage : l'onglet Calendrier cesse
de laisser des équipes sans match dans plus de la moitié des tirages de
mi-saison, et « régénérer » cesse de rendre exactement le même appariement.

## Conception

Le use case construit `DrawInput` à partir de ce qu'il a déjà sous la main :

| Champ | Source |
|---|---|
| `equipes` | les équipes engagées, comme aujourd'hui |
| `historique` | les rencontres de la saison, **comptées par paire** avec leur dernière journée |
| `interdites` | `ITeamInfoPort` — deux équipes d'un même coach (R10) |
| `jamais_exemptees` | vide : le Calendrier ne tient pas d'historique d'exemption |

`interdites` **s'applique aussi au Calendrier**, et ce n'est pas une extension de
périmètre déguisée : un coach ne joue pas contre lui-même, que la rencontre
vienne d'un tirage manuel ou d'un sondage. La règle est la même des deux côtés.

`jamais_exemptees` vide veut dire que l'exemptée est tirée uniformément — le
comportement actuel, ni meilleur ni pire. C'est le sondage qui apportera
l'historique.

## Le point d'attention — la suppression

`generate_round_pairings` est retirée de `match_day.rs`. Avant de supprimer,
**lister ses consommateurs** (règle 4 du CLAUDE.md) : à la dernière
vérification, `generate_pairings` et `generate_all_pairings`, plus ses six
tests. Ceux-ci portent sur des propriétés — `len`, pas de doublon — et non sur
des appariements nommés : **ils survivent tels quels** sous le nouvel
algorithme, et c'est ce qui rend la migration sûre.

## Checklist

- [ ] Construire `RencontresJouees` depuis les rencontres de la saison
- [ ] Construire `interdites` depuis `ITeamInfoPort`
- [ ] `generate_pairings` et `generate_all_pairings` appellent `tirer`
- [ ] Lister les consommateurs de `generate_round_pairings`, puis la supprimer
- [ ] Ses six tests migrent vers `tirer` et **passent sans être réécrits**
- [ ] Un test de non-régression : régénérer deux fois une même journée donne
      deux appariements différents à effectif suffisant
- [ ] `make lint`, `make check-arch`, `make test`
