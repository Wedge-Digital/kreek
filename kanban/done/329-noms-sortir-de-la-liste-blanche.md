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

## Mise à jour du 2026-08-25 — ce que le commit `542bdfd` a déjà réglé

La carte décrivait **six règles de nommage incohérentes**, une par value object.
Ce constat est périmé : `542bdfd` (« un charset unique pour tout le texte
saisi ») les a fondues en deux expressions, dans
`src/app/shared_kernel/identity/charset.rs` :

| Constante | Portée | Expression |
|---|---|---|
| `TEXTE_SAISI` | tout texte saisi — compétence, poste, joueur, équipe, roster, espace, saison, journée, tier, compétition | `^[\p{L}\p{M}\p{N} '’\-–—.,;:!?()\[\]«»"“”…&@#%*+=_°~/\\]+$` |
| `IDENTIFIANT_COACH` | `CoachName` seul | `^[\p{L}\p{M}\p{N}._ '’\-–—]+$` |

Les trois exemples qui ouvraient cette carte **passent désormais tous les
trois** : `L'Ost du Chaos`, `Les Zazous & Cie`, `Jean-Éric O'Brien`. Le commit a
pris pour base l'ancien charset de `CompetitionName`, le plus permissif des
onze, et y a ajouté les marques combinantes (`\p{M}`, pour les accents
décomposés collés depuis macOS), les apostrophes et tirets typographiques, les
guillemets et deux séparateurs.

## Le problème, tel qu'il reste

**C'est toujours une liste blanche**, et elle se rouvrira au prochain caractère
oublié. Restent dehors aujourd'hui :

| Refusé | Exemple qui échoue |
|---|---|
| `\p{So}` — emoji et symboles | `Team 🏈` |
| `\p{Sc}` — symboles monétaires | `Ligue €uro` |
| `< > \| { } $ ^` | `Équipe <Étoilée>`, `Journée\|1` |

Le mécanisme n'a pas changé : chaque caractère non prévu redevient une carte, et
un nom refusé **ne dit toujours pas pourquoi** — les quatre sites qui avalent
l'échec (`UnknownSkill` dans `validate_customisation_use_case.rs`, poste replié
sur « Joueur » dans `player_creation.rs`, roster escamoté par un `.ok()?` deux
fois dans `roster_service.rs`) sont intacts. `CLAUDE.md` le note noir sur blanc :
« élargir le charset fait passer le français d'aujourd'hui ; ça ne répare pas le
mécanisme ».

**Ce qui reste vrai de la divergence** : `NameVo` porte toujours **quatre alias**
— `TeamName` (dans `team_creation`), `SeasonName`, `MatchDayName`, `TierName` —
qui sont littéralement le même type Rust, et sa borne est 50 quand le `TeamName`
de `teams` est à 100. Le charset, lui, est désormais commun aux deux.

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

La règle vit à un seul endroit — et ce fichier **existe déjà** depuis
`542bdfd` : `identity/charset.rs`. La bascule y remplace le contenu des deux
constantes, elle n'en crée pas un troisième :

```rust
use regex::Regex;
use std::sync::LazyLock;

pub static TEXTE_SAISI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^\p{Cc}\p{Zl}\p{Zp}\x{202A}-\x{202E}\x{2066}-\x{2069}]+$").unwrap()
});
```

Les `nutype` la référencent déjà par `validate(regex = TEXTE_SAISI)` : **aucun
d'eux n'est à toucher**, ce que la version d'origine de cette carte ne pouvait
pas prévoir. Le câblage est fait ; il ne reste que l'expression à retourner.

Les deux points d'intendance sont vérifiés : `nutype` 0.7 accepte
`regex = IDENT` sur un `static LazyLock<Regex>` (README, section *regex*), et
`regex` est déjà une dépendance directe (`Cargo.toml:24`). Aucune dépendance
nouvelle. La regex ci-dessus a été compilée et éprouvée sur les douze cas du
tableau de tests plus bas.

## Piège n°1 — il y a deux `TeamName`, et ils ne portent plus la même borne

`team_creation` utilise `NameVo` (50), `teams` a le sien (100). Depuis
`542bdfd`, **les deux partagent `TEXTE_SAISI`** : le scénario d'origine — un
seul des deux assoupli, et une équipe fraîchement créée s'appelant
silencieusement « Unknown » via
`teams/io/app_events/team_created_listener.rs:66` — ne peut plus se produire par
le charset. Il le pourrait encore par la **longueur**, si l'un des deux bougeait
seul. Les faire converger à 100 reste au programme.

```rust
let tname = TeamName::try_new(team_name)
    .unwrap_or_else(|_| TeamName::try_new("Unknown".to_string()).unwrap());
```

Ce repli silencieux reste la vraie fragilité : il transformera n'importe quelle
divergence future en nom « Unknown », sans une ligne de journal.

Au passage : `pub type TeamName = NameVo` est un **alias**, pas un newtype —
`TeamName`, `SeasonName`, `MatchDayName` et `TierName` sont aujourd'hui le même
type Rust, et le compilateur accepte une saison là où une équipe est attendue.
Puisqu'on ouvre ce fichier, les séparer en quatre `nutype` distincts partageant
le charset. C'est la règle « pas de type primitif nu » du `CLAUDE.md`, un cran
au-dessus.

## Piège n°2 — `CoachName` n'est pas un nom d'affichage, c'est un identifiant de connexion

`post_login` s'en sert comme identifiant. L'unicité est portée par
`users_coach_name_lower_uq UNIQUE (lower(coach_name))` — depuis la migration
`20260812000001`, elle est **insensible à la casse**, mais reste **octet par
octet** pour tout le reste.
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

## Piège n°3 — un rendu déjà cassé, depuis `542bdfd`

`competitions/io/web/templates/admin/widgets/schedule-round-detail.html:102` :

```js
{ value: '{{ t.team_id }}', text: '{{ t.team_name }}', coach: '{{ t.coach_name }}', … },
```

C'est à l'intérieur d'un `<script>`. Askama échappe en entités HTML, que le
navigateur **ne décode pas** dans un `<script>` : `L'Ost` s'affiche
`L&#x27;Ost` dans le sélecteur d'équipes de l'admin. Ce n'est pas une injection
— l'échappement tient, `</script>` devient `&lt;/script&gt;`.

**Ce défaut n'est plus à venir : il est actif.** La carte le prévoyait « le jour
où la carte sort » ; depuis `542bdfd`, l'apostrophe est acceptée partout, donc
tout nom d'équipe ou de coach qui en porte une s'affiche corrompu dans ce
sélecteur, aujourd'hui, en production.

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

`RosterName` — le seul qui n'ait **aucune** regex, ni avant ni après
`542bdfd`.

`CompetitionName`, `PositionNameVo`, `PersonalName` et `SkillName` sortent du
hors-périmètre : depuis `542bdfd` ils partagent tous `TEXTE_SAISI`, donc la
bascule les emporte qu'on le veuille ou non. C'est un élargissement de portée
par rapport à la version d'origine de la carte, et il faut le tester en
conséquence.

Les longueurs restent inchangées, à l'exception des deux `TeamName` qui
convergent : uniformiser les bornes est un autre sujet que les caractères.

Rien à migrer en base : les colonnes sont `VARCHAR(100)`/`TEXT` sans contrainte
de charset, et on élargit — tout nom existant reste valide.

## Tableau de tests unitaires

Une assertion par ligne, à décliner sur chaque VO. La colonne « déjà »
distingue ce que `542bdfd` fait passer aujourd'hui — utile en non-régression,
mais qui ne prouve rien de la bascule — de ce qui la discrimine réellement.

| Valeur | Attendu | Déjà ? | Motif |
|---|---|---|---|
| `L'Ost du Chaos` | accepté | oui | apostrophe |
| `Les Zazous & Cie` | accepté | oui | esperluette |
| `F.C. Machin` | accepté | oui | points |
| `Ligue « Hiver » — 2026` | accepté | oui | guillemets français, tiret cadratin |
| `Скавены` | accepté | oui | `\p{L}` couvre déjà les alphabets non latins |
| `Équipe <Étoilée>` | accepté | **non** | chevrons — l'échappement au rendu s'en charge |
| `Ligue €uro` | accepté | **non** | `\p{Sc}`, hors de la liste blanche actuelle |
| `Journée\|1` | accepté | **non** | la barre verticale, refusée aujourd'hui |
| `Team 🏈` | accepté | **non** | emoji simple |
| `Famille 👨‍👩‍👧` | accepté partout **sauf `CoachName`** | séquence ZWJ (U+200D) |
| `Ligne1\nLigne2` | refusé | `\p{Cc}` |
| `Tab\there` | refusé | `\p{Cc}` |
| `evil\u{202E}nom` | refusé | override bidi |
| `sep\u{2028}ligne` | refusé | `\p{Zl}` |
| `   ` | refusé | vide après trim |
| 101 caractères | refusé | borne de longueur |

## Checklist

- [x] Les deux constantes de `identity/charset.rs` basculées en liste noire
- [x] `CoachName` ajoute le refus de `\p{Cf}` — les invisibles, et eux seuls
- [x] `NameVo` éclaté, puis **supprimé** : plus aucun consommateur
- [x] Les deux `TeamName` portent la même règle et la même longueur (100)
- [x] Tests unitaires : le tableau ci-dessus, plus ce que la bascule ouvre
- [x] Test unitaire : un nom valide à la source l'est dans `teams` — le
      « Unknown » du listener ne peut plus se déclencher par le charset
- [x] `schedule-round-detail` passe par `data-teams` + `JSON.parse`
- [x] Messages d'erreur de `register_space` et `post_draft_team` réécrits
- [x] Test e2e, **vu échouer** : `tests/e2e/test_noms_typographiques.py`
- [x] `make test` — 1239 tests
- [x] `make lint`, `make check-arch`

### Les noms de la checklist d'origine n'existaient plus

Elle demandait un fichier `identity/display_name_charset.rs` et une constante
`DISPLAY_NAME`. Le commit `542bdfd` avait déjà créé `identity/charset.rs` avec
`TEXTE_SAISI` et `IDENTIFIANT_COACH` ; en introduire un troisième nom aurait
rouvert la dispersion que ce commit venait de fermer.

## Ce qui a été fait, et ce que la carte n'avait pas vu

**Cinq types, pas trois.** `NameVo` n'était pas qu'un alias derrière quatre
noms : `competition_structure.rs` l'employait **directement** pour le nom d'un
groupe de classement et pour celui des deux variantes de `ScheduledDate`. Ce
dernier est sémantiquement une journée — il devient `MatchDayName`. Le groupe
gagne son `RankingGroupName`. `NameVo` a donc pu disparaître entièrement.

**Le compilateur a trouvé trois confusions**, toutes dans du code de test, et
toutes du même genre : un `NameVo` posé là où une journée est attendue. Elles
n'étaient visibles qu'après l'éclatement — c'est exactement ce qu'un alias
partagé empêche de voir.

**`cargo build` ne suffisait pas** à le constater : il ne compile pas le code
sous `#[cfg(test)]`. Zéro erreur y donnait une fausse assurance ; `cargo build
--tests` en a rendu trois.

**Huit tests unitaires affirmaient l'ancienne règle** — `|`, `<script>`,
`Bag@uze` refusés. Réécrits pour affirmer la nouvelle, et pour tenir ce que la
bascule **ouvre** : c'est cette moitié-là qui distingue la liste noire de
l'ancienne liste blanche.

### Le point à trancher, laissé ouvert

La carte décidait que `CoachName` reçoive le charset commun moins `\p{Cf}`.
Appliqué. Mais `charset.rs` excluait `@` **délibérément**, pour qu'un
pseudonyme ne prenne pas l'allure d'une adresse électronique — un raisonnement
que la carte ne mentionnait pas. `Bag@uze` est donc devenu un pseudonyme
valide. Le retour arrière tient en un caractère de classe.
