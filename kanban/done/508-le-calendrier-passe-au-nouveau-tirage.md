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

## Ce que le branchement a révélé — les fixtures e2e

**Le tirage devenu aléatoire a rendu la suite e2e non déterministe.** Quatre
tests sont tombés, chacun à un passage différent : trois de classement au
premier, un de journalier au second, une fois les premiers corrigés.

La cause est une hypothèse que personne n'avait écrite. Le Calendrier crée un
**brouillon de rapport par appariement**, avec l'orientation de l'appariement ;
`create_match_report_use_case` retrouve ce brouillon par
`find_id_by_round_and_teams` et le confirme, plutôt que d'en créer un second.
L'orientation demandée à `/match-report/new` est donc ignorée dès qu'un
appariement existe.

C'est **correct** — le calendrier décide qui reçoit, pas celui qui ouvre le
rapport. Mais l'ancien tirage, glouton dans l'ordre des indices, faisait toujours
recevoir `teams[0]` : les fixtures qui écrivaient `home=teams[0]` tombaient juste
par construction. Quatorze fichiers de test posaient un côté en dur **et**
généraient un calendrier.

### La correction : ne plus générer d'appariements sans raison

`build_full_competition` prend `with_pairings`, **faux par défaut**.
`sync_and_generate_schedule` se scinde en `sync_schedule` — les journées, vides —
et `generate_pairings`, qui n'est appelée que par les tests portant réellement
sur le calendrier : `test_pairing_deletion`, `test_competition_matchs`,
`test_team_matches`, `test_team_treasury_tab`, `test_annuler_un_rapport`,
`test_competition_admin_acces`.

Sans appariement préexistant, aucun brouillon à retrouver : l'orientation
demandée est respectée, et le déterminisme revient. **Un test sur le barème SPP
n'a aucune raison de dépendre d'un tirage au sort.**

`play_match` est corrigé au passage : `home_team_id` désigne désormais
l'**équipe**, pas le côté — tout ce que l'appelant préfixe `home_` la suit, où
qu'elle atterrisse. Le helper promettait déjà cela ; il lisait le côté `home`.

**Aucune assertion n'a été affaiblie.** Deux passages complets consécutifs,
368 tests passés chacun.

## Checklist

- [ ] Construire `RencontresJouees` depuis les rencontres de la saison
- [ ] Construire `interdites` depuis `ITeamInfoPort`
- [ ] `generate_pairings` et `generate_all_pairings` appellent `tirer`
- [ ] Lister les consommateurs de `generate_round_pairings`, puis la supprimer
- [ ] Ses six tests migrent vers `tirer` et **passent sans être réécrits**
- [ ] Un test de non-régression : régénérer deux fois une même journée donne
      deux appariements différents à effectif suffisant
- [ ] `build_full_competition(with_pairings=False)` par défaut, et le drapeau
      posé sur les six tests qui portent sur le calendrier
- [ ] `play_match` suit l'équipe, pas le côté
- [ ] **Deux** passages de `make test-impacted` d'affilée, pour prouver le
      déterminisme retrouvé
- [ ] `make lint`, `make check-arch`, `make test`
