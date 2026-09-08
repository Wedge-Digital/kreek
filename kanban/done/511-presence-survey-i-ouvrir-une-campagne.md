# `PresenceSurvey` I — ouvrir une campagne

**Priorité : haute — l'agrégat que les deux cartes suivantes complètent**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien — **c'est elle qui ouvre la vague du socle**, avant la 510
**Fichiers :** `src/app/competitions/domain/presence_survey.rs` (nouveau),
`src/app/competitions/domain/error.rs`
**Spec :** `docs/specs/sondage-presence/onglet-presences/06-domaine.md`

## L'objectif

La forme de l'agrégat, ses value objects, sa construction et ses requêtes de
lecture. Les commandes viennent en 512 et 513.

L'agrégat a été conçu **d'un bloc pour les trois unités** — l'organisateur, le
jeton, l'encart du coach connecté — parce que la troisième méthode qu'on greffe
révèle souvent que les deux premières avaient la mauvaise signature.

## La forme

```rust
pub struct PresenceSurvey {
    id:          SurveyId,
    season_id:   SeasonId,
    round_id:    MatchId,
    deadline:    SurveyDeadline,
    auto_remind: AutoRemind,
    opened_at:   OpenedAt,
    fermeture:   Fermeture,
    reponses:    Vec<Reponse>,
    appariement: Appariement,
}

pub enum Fermeture   { Aucune, Decidee { le: FermeeLe } }
pub enum Appariement { Aucun, Fait { exemptee: Option<TeamId> } }

pub struct Reponse { team_id: TeamId, coach_id: CoachId, token: SurveyToken, presence: Presence }

pub enum Presence {
    SansReponse,
    Declaree { venue: Venue, le: ReponduLe, par: Repondant },
}
pub enum Repondant { Jeton, Coach(CoachId), Organisateur(CoachId) }   // R28
```

**Aucun champ n'est `pub`.** Le seul chemin d'écriture d'une `Presence` est
`enregistrer` (carte 512), qui porte trois règles ensemble ; un `pub` sur
`reponses` les rendrait contournables par un `survey.reponses[0].presence = ...`
que ni le compilateur, ni `check-arch`, ni la revue ne signaleraient.

**`Presence` est un enum à données portées, jamais un booléen nullable.**
L'horodatage et l'auteur n'ont de sens que là où une réponse existe ; un enum
plat avec deux `Option` à côté remplacerait un `Option` par deux et laisserait
construire une réponse déclarée sans horodatage.

**`Repondant` a trois variantes pour trois chemins** (R28) : le jeton reçu par
e-mail, le coach connecté depuis l'encart, l'organisateur depuis l'onglet. Elles
n'ont pas la même autorisation — le jeton *est* l'autorisation, la session ne
l'est pas — et c'est la carte 512 qui pose le contrôle.

**Le canal ne se persiste pas** : `saisi_par_admin` garde ses deux cas, et un
`NULL` relu rend `Coach(coach_id de la réponse)`. R6 distingue l'organisateur du
coach, jamais le jeton de l'encart.

**`SurveyToken` n'est pas un `EntityId`**, bien qu'il en ait la forme : un
identifiant technique et un secret d'URL n'ont ni le même cycle de vie ni les
mêmes règles de divulgation. Les confondre autoriserait `survey.id` là où le
jeton est attendu, sans un mot du compilateur.

## R23 — la clôture est calculée, jamais subie

```rust
pub fn statut_de(fermeture: &Fermeture, deadline: &SurveyDeadline, maintenant: ...) -> SurveyStatus
```

| `fermeture` | échéance | statut |
|---|---|---|
| `Aucune` | à venir | `Ouverte` |
| `Aucune` | passée | `Close(Echeance)` |
| `Decidee { le }` | quelconque | `Close(Decision)` |

**Fonction libre, pas seulement une méthode.** La barre latérale (carte 519) lit
des DTOs, pas des agrégats, et doit répondre à la même question. Une règle
calculée à deux endroits finit par l'être de deux façons — et celle-ci croise
deux champs et une horloge, assez pour diverger.

Écarté : une tâche planifiée qui clorait les campagnes échues. Une campagne
serait restée ouverte jusqu'à vingt-quatre heures après son échéance, ses liens
répondant pendant ce temps, contre ce que l'e-mail annonce noir sur blanc.

## `ouvrir`

```rust
ouvrir(id, season, round: &MatchDay, destinataires, deadline, auto_remind, maintenant)
    -> Result<Self, DomainError>
```

Reçoit **`&MatchDay`**, pas un identifiant : « peut-on sonder cette journée ? »
est une question métier, la répondre dans le use case l'aurait sortie du
domaine. Engendre une `Reponse` par équipe destinataire (R1) avec son jeton et
`Presence::SansReponse`, **y compris pour les coachs sans adresse** — R3 dit que
leur équipe entre dans la campagne, seul l'e-mail manque.

## Checklist

- [ ] Les value objects : `SurveyId`, `SurveyToken`, `SurveyDeadline`,
      `AutoRemind`, `ReponduLe`, `Venue`, `Repondant`, `Presence`, `SurveyStatus`
- [ ] La struct, `Fermeture`, `Appariement`, `Reponse` — aucun champ `pub`
- [ ] `ouvrir`, `rehydrater`
- [ ] `statut_de` libre + `statut(maintenant)` qui l'appelle
- [ ] Requêtes : `presents`, `compte_presents`, `compte_absents`,
      `compte_sans_reponse`, `engagees`, `sans_reponse`, `reponse_par_jeton`,
      `peut_tirer`
- [ ] `DomainError` : `SurveyOnRestDay`, `NotEnoughPresent { presents }`
- [ ] Tests : R1 (un coach à deux équipes, deux jetons distincts) · R2 (journée
      de repos refusée) · R3 (sans adresse entre quand même) · R5 (`presents`
      exclut les `SansReponse`) · R15 (`peut_tirer` à un présent) ·
      R23 (`Close(Echeance)` sans `Decidee`)
- [ ] `make lint`, `make check-arch`, `make test`
