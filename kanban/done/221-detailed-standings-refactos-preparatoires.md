# Classement détaillé — Refactos préparatoires

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/ranking/io/web/builders.rs`, `src/app/ranking/io/web/widgets/classement_widget.rs`, `src/app/ranking/use_cases/standings_service.rs`
**Spec :** `docs/specs/ranking/tiebreakers/detailed-standings/{03-back,07-integration}.md`

## Objectif

Préparer le terrain du nouvel onglet **sans rien changer au comportement**. Deux
extractions indépendantes, réunies parce qu'elles ont la même signature : rendre
partageable ce qui est aujourd'hui enfermé.

**Critère d'acceptation** : les tests existants restent verts **sans être modifiés**.
C'est le meilleur filet possible pour ce genre de déplacement — si un test doit être
retouché, c'est que le comportement a bougé.

## Conception

### 1. Extraire le découpage par poule (`builders.rs`)

`build_classement_groups` porte aujourd'hui le découpage (poule unique ou absente, une
poule par groupe, section « Non assignées ») **mélangé au rendu** des lignes du classement
simple. Le nouvel onglet a besoin du même découpage avec un rendu différent.

```rust
/// Une poule et les données qui la concernent — le découpage seul, sans rendu.
struct GroupSlice {
    title: Option<String>,
    lines: Vec<RankingLineRow>,
    teams: Vec<EnrolledTeamInfo>,
}

fn split_into_groups(
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
) -> Vec<GroupSlice>;
```

`build_classement_groups` devient un `map` sur le résultat. Sans cette extraction, la
règle « chaque poule est un classement autonome » serait implémentée deux fois et pourrait
diverger.

Les **six tests de découpage** de `builders.rs` (poule unique, poules multiples, poule
vide, équipes non assignées, absence de section non assignées, classement à plat) couvrent
exactement ce comportement — ils ne doivent pas bouger.

### 2. Déplacer `tiebreak_order_of` vers `standings_service`

Aujourd'hui **privé** dans `classement_widget.rs`. C'est le même mappage port → domaine
que `to_tiebreak_order`, avec la gestion de l'`Option` en plus — il appartient au service.

Ses trois tests suivent le déplacement, inchangés.

Sans ce déplacement, les deux widgets construiraient l'ordre de départage chacun de leur
côté : l'onglet détaillé pourrait afficher des colonnes ne correspondant pas à l'ordre
réellement appliqué par l'onglet simple, sans que rien ne le signale.

## Déplacement de code — rappel

Règle 5 du CLAUDE.md : **copier-coller exact**, adapter uniquement les imports et les
références. Ne rien réécrire de mémoire.

## Checklist

- [ ] `split_into_groups` extrait, `build_classement_groups` s'appuie dessus
- [ ] `tiebreak_order_of` déplacé dans `standings_service`, ses 3 tests avec lui
- [ ] Les 6 tests de découpage **inchangés** et verts
- [ ] Aucun changement de comportement du widget Classement
- [ ] `make test` + `make check-arch` passent
