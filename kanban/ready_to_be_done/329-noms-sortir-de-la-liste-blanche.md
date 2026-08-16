# Noms d'espace, d'équipe, de coach, de saison et de tier — sortir de la liste blanche

**Priorité : moyenne** — rien n'est cassé, mais la règle refuse des noms
légitimes et diverge d'un BC à l'autre
**Dépend de :** rien
**Fichiers :** `src/app/shared_kernel/identity/{space_name.rs, coach_name.rs,
name_vo.rs}`, `src/app/shared_kernel/bloodbowl/{team.rs, season_name.rs,
tier.rs}`, `src/app/teams/domain/value_objects.rs`,
`src/app/spaces/io/web/register_space.rs`,
`src/app/team_creation/io/web/post_draft_team.rs`,
`src/app/competitions/io/web/templates/admin/widgets/schedule-round-detail.html`
(+ son contrôleur)

## Le problème

Il n'y a pas une règle de nommage mais six, incohérentes.

| Value object | Fichier | Long. | Charset |
|---|---|---|---|
| `SpaceName` | `identity/space_name.rs:6` | 100 | `\p{L} 0-9 _ - ' .` espace |
| `NameVo` — alias de `TeamName` (`team_creation`), `SeasonName`, `TierName` | `identity/name_vo.rs:5` | 50 | `\p{L} 0-9 -` espace |
| `TeamName` (BC `teams`) | `teams/domain/value_objects.rs:34` | 100 | `\p{L} 0-9 -` espace |
| `CoachName` | `identity/coach_name.rs:5` | 50 | `\p{L} 0-9 . _ -` espace |
| `CompetitionName` | `bloodbowl/competition_name.rs:8` | 100 | très large |
| `RosterName` | `teams/domain/value_objects.rs:22` | 100 | aucune regex |

`L'Ost du Chaos` est refusé en nom d'équipe mais accepté en nom d'espace.
`Les Zazous & Cie` est refusé des deux côtés, accepté en compétition.
`Jean-Éric O'Brien` est refusé en nom de coach.

## Pourquoi la liste blanche, et pas une liste blanche plus longue

Elle se rouvre à chaque caractère oublié : `CompetitionName`, la plus
permissive du projet, refuse encore `—`, `«»`, `/`, `|`, `"`. Chaque oubli
devient une carte.

Surtout : **une liste blanche de caractères n'est pas la défense contre
l'injection.** Celle-ci est l'échappement au rendu, déjà en place — Askama
échappe par défaut. Garder les deux, c'est payer une contrainte utilisateur
pour une sécurité qu'on a déjà ailleurs.

## La règle cible

`sanitize(trim)`, `not_empty` (donc au moins un caractère non blanc après
trim), longueur bornée, et une regex qui **refuse** au lieu d'autoriser :

```
^[^\p{Cc}\p{Zl}\p{Zp}\x{202A}-\x{202E}\x{2066}-\x{2069}]+$
```

| Refusé | Pourquoi |
|---|---|
| `\p{Cc}` — contrôles C0/C1, dont `\n` `\r` `\t` | un nom est une ligne ; un `\n` casse les logs, les en-têtes et les exports |
| `\p{Zl}` `\p{Zp}` — U+2028/2029 | même raison, hors de `Cc` |
| U+202A–202E, U+2066–2069 — overrides bidi | un nom peut sinon s'afficher à l'envers de ce qu'il contient (Trojan Source appliqué à l'UI) |

Passent désormais : `'` `&` `"` `<` `>` `!` `?` `/` `|` `«»` `—` `€`, les
emoji, les alphabets non latins. U+200D (ZWJ) reste **autorisé** — le refuser
casserait les séquences emoji composées et plusieurs écritures indiennes.

La règle vit à un seul endroit, `identity/display_name_charset.rs` :

```rust
use regex::Regex;
use std::sync::LazyLock;

pub static DISPLAY_NAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^\p{Cc}\p{Zl}\p{Zp}\x{202A}-\x{202E}\x{2066}-\x{2069}]+$").unwrap()
});
```

et chaque `nutype` la référence par `validate(regex = DISPLAY_NAME)`.

Les deux points d'intendance sont vérifiés : `nutype` 0.7 accepte
`regex = IDENT` sur un `static LazyLock<Regex>` (README, section *regex*), et
`regex` est déjà une dépendance directe (`Cargo.toml:24`). Aucune dépendance
nouvelle. La regex ci-dessus a été compilée et éprouvée sur les douze cas du
tableau de tests plus bas.

## Piège n°1 — il y a deux `TeamName`, et ils doivent bouger ensemble

`team_creation` utilise `NameVo` (50), `teams` a le sien (100).
`teams/io/app_events/team_created_listener.rs:66` fait :

```rust
let tname = TeamName::try_new(team_name)
    .unwrap_or_else(|_| TeamName::try_new("Unknown".to_string()).unwrap());
```

Assouplir un seul des deux, et une équipe fraîchement créée s'appelle
**silencieusement « Unknown »** dans le BC `teams`. Les deux convergent à 100.

Au passage : `pub type TeamName = NameVo` est un **alias**, pas un newtype —
`TeamName`, `SeasonName` et `TierName` sont aujourd'hui littéralement le même
type Rust, et le compilateur accepte une saison là où une équipe est attendue.
Puisqu'on ouvre ce fichier, les séparer en trois `nutype` distincts partageant
le charset. C'est la règle « pas de type primitif nu » du `CLAUDE.md`, un cran
au-dessus.

## Piège n°2 — `CoachName` n'est pas un nom d'affichage, c'est un identifiant de connexion

`post_login` s'en sert comme identifiant, et
`migrations/20260502120000_users.sql:10` porte
`users_coach_name_uq UNIQUE (coach_name)` — unicité **octet par octet**.
Rendre ce champ pleinement permissif ouvre l'usurpation par caractère
invisible : `Bagouze` et `Bagouze` avec un ZWSP inséré deviennent deux comptes
distincts, visuellement identiques, dans la liste des coachs comme dans les
résultats de match.

**Décision retenue :** `CoachName` applique le charset commun **plus** le refus
de `\p{Cf}` (invisibles et jointures). Coût : pas d'emoji composé dans un nom
de coach — acceptable pour un identifiant. Les homoglyphes inter-alphabets
(`а` cyrillique contre `a` latin) restent possibles ; ils demandent une
normalisation et une table de confusables, c'est une carte à part si le besoin
se présente.

## Piège n°3 — un rendu qui va casser le jour où la carte sort

`competitions/io/web/templates/admin/widgets/schedule-round-detail.html:102` :

```js
{ value: '{{ t.team_id }}', text: '{{ t.team_name }}', coach: '{{ t.coach_name }}', … },
```

C'est à l'intérieur d'un `<script>`. Askama échappe en entités HTML, que le
navigateur **ne décode pas** dans un `<script>` : `L'Ost` s'affichera
`L&#x27;Ost` dans le sélecteur d'équipes de l'admin. Ce n'est pas une injection
— l'échappement tient, `</script>` devient `&lt;/script&gt;` — mais c'est une
corruption visible qui n'apparaîtra qu'après cette carte, donc elle en fait
partie.

Correctif : cesser d'interpoler dans du JS. Le contrôleur sérialise le tableau
avec `serde_json`, le template le pose en attribut `data-teams`, le script fait
`JSON.parse(el.dataset.teams)` — un attribut, lui, est bien décodé par le
navigateur. Le précédent existe : `new-competition-phase-1.html:72`
(`initial_admin_ids_json`).

*Vérifié par ailleurs* : aucun de ces cinq noms n'est interpolé dans un
attribut porteur de JS (`@click`, `onclick`) — ce cas-là serait une vraie
rupture de syntaxe, pas un défaut d'affichage. Le seul site du genre,
`mercenary-selector-widget.html:20`, porte un nom de position
(`PositionNameVo`), hors périmètre. Aucune requête `LIKE`/`ILIKE` dans le
projet, donc pas de sujet côté recherche.

## Piège n°4 — les messages d'erreur mentent déjà

`register_space.rs:95` annonce « lettres, chiffres, tirets et underscores »
alors que l'apostrophe et le point passent. `post_draft_team.rs:47` annonce
« 1–50 caractères alphanumériques ». Les deux sont à réécrire d'après la règle
réelle.

## Hors périmètre

`CompetitionName`, `RosterName`, `PositionNameVo`, `PersonalName`, `SkillName`.
La même bascule leur ira, mais l'élargir maintenant double la surface de test
sans rien débloquer. `CompetitionName` deviendra d'ailleurs un simple appel à
`DISPLAY_NAME` — carte de suite, triviale.

Les longueurs restent inchangées, à l'exception des deux `TeamName` qui
convergent : uniformiser les bornes est un autre sujet que les caractères.

Rien à migrer en base : les colonnes sont `VARCHAR(100)`/`TEXT` sans contrainte
de charset, et on élargit — tout nom existant reste valide.

## Tableau de tests unitaires

Une assertion par ligne, à décliner sur chaque VO.

| Valeur | Attendu | Motif |
|---|---|---|
| `L'Ost du Chaos` | accepté | apostrophe |
| `Les Zazous & Cie` | accepté | esperluette |
| `F.C. Machin` | accepté | points |
| `Équipe <Étoilée>` | accepté | chevrons — l'échappement au rendu s'en charge |
| `Ligue « Hiver » — 2026` | accepté | guillemets français, tiret cadratin |
| `Скавены` | accepté | alphabet non latin |
| `Team 🏈` | accepté | emoji simple |
| `Famille 👨‍👩‍👧` | accepté partout **sauf `CoachName`** | séquence ZWJ (U+200D) |
| `Ligne1\nLigne2` | refusé | `\p{Cc}` |
| `Tab\there` | refusé | `\p{Cc}` |
| `evil\u{202E}nom` | refusé | override bidi |
| `sep\u{2028}ligne` | refusé | `\p{Zl}` |
| `   ` | refusé | vide après trim |
| 101 caractères | refusé | borne de longueur |

## Checklist

- [ ] `identity/display_name_charset.rs` : `DISPLAY_NAME` en `LazyLock<Regex>`
- [ ] `SpaceName` (100), `CoachName` (50, + refus `\p{Cf}`), `SeasonName` (50),
      `TierName` (50) basculés sur `DISPLAY_NAME`
- [ ] `NameVo` éclaté : `TeamName`, `SeasonName`, `TierName` deviennent trois
      `nutype` distincts ; `NameVo` disparaît si plus aucun consommateur
      (règle 4 du `CLAUDE.md` — lister les consommateurs avant de supprimer)
- [ ] Les deux `TeamName` portent la même règle et la même longueur (100)
- [ ] Tests unitaires par VO : le tableau ci-dessus
- [ ] Test unitaire : un nom valide en `team_creation` l'est en `teams` — le
      « Unknown » du listener ne peut plus se déclencher
- [ ] `schedule-round-detail` passe par `data-teams` + `JSON.parse`
- [ ] Messages d'erreur de `register_space` et `post_draft_team` réécrits
- [ ] Test e2e : créer un espace puis une équipe nommés `L'Ost & Cie`, vérifier
      l'affichage sur la page d'équipe **et** dans le sélecteur d'admin du
      calendrier
- [ ] `make test` passe
- [ ] `make lint` passe
- [ ] `make check-arch` passe
