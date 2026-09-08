# La table des campagnes et son dépôt

**Priorité : haute — tout le reste charge et persiste par là**
**Épic :** E16 — Sondage de présence
**Dépend de :** 511, qui crée `PresenceSurvey` — le port le rend, il ne compile
pas sans lui. Les deux cartes s'annonçaient sans dépendance ; c'était faux, et
corrigé au moment d'attaquer la vague.
**Fichiers :** `migrations/20260907000001_competition_presence_surveys.sql`,
`src/app/competitions/domain/presence_survey_repository_port.rs`,
`src/app/competitions/io/repository/presence_survey_repository.rs`,
`src/app/competitions/io/repository/sql/presences/*.sql`
**Spec :** `docs/specs/sondage-presence/onglet-presences/07-integration.md`

## L'objectif

Deux tables, un port à trois méthodes, cinq requêtes, et les tests
d'intégration qui prouvent que ce sont les **contraintes** qui garantissent, pas
le code applicatif.

## Le schéma

```
competition_presence_surveys
    id, season_id, round_id, deadline, auto_remind, opened_at
    close_le TIMESTAMPTZ NULL                          <- Fermeture::Decidee
    UNIQUE (round_id)                                  <- R2

competition_presence_answers
    id, survey_id, team_id, coach_id, token
    presence TEXT NOT NULL CHECK (presence IN ('sans_reponse','presente','absente'))
    repondu_le      TIMESTAMPTZ NULL
    saisi_par_admin TEXT NULL                          <- R6
    UNIQUE (survey_id, team_id)                        <- R1
    UNIQUE (token)
    CHECK (
      (presence  = 'sans_reponse' AND repondu_le IS NULL AND saisi_par_admin IS NULL)
      OR (presence <> 'sans_reponse' AND repondu_le IS NOT NULL)
    )
```

**Pas de colonne `statut`** : R23 le calcule depuis l'échéance et `close_le`.
Une colonne aurait pu dire « ouverte » sur une campagne échue, et rien n'aurait
signalé la divergence.

**Pas de table de jetons** : R7 leur refuse toute échéance propre, ils n'ont
rien à porter qu'une réponse ne porte déjà.

**Le `CHECK` n'est pas une ceinture de plus.** Postgres n'a pas de type somme :
il est le seul endroit où l'impossibilité d'une réponse déclarée sans
horodatage s'écrit côté base. *La contrainte garantit, le type exprime.*

## Le port

```rust
#[async_trait]
pub trait IPresenceSurveyRepository: Send + Sync {
    async fn find_by_round(&self, round_id: &str)
        -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;
    async fn save(&self, survey: &PresenceSurvey)
        -> Result<(), PresenceSurveyRepositoryError>;
    async fn list_summaries(&self, season_id: &str)
        -> Result<Vec<SurveySummaryDto>, PresenceSurveyRepositoryError>;
}
```

**Trois méthodes, pas quatre.** `find_by_token` appartient à l'unité
`reponse-coach` et y sera ajoutée : déclarer ici une méthode qu'aucun appelant
n'utilise ferait porter à cette carte une décision qui n'est pas la sienne.

**`save` réécrit tout** — la campagne et ses réponses, en une transaction, même
pour un simple changement de présence. C'est le prix d'un **chemin d'écriture
unique** vers la colonne `presence` : un `save_answer` ciblé ouvrirait une
seconde porte, et R19, R13 et R21 tiennent précisément parce qu'il n'y en a
qu'une. Quatorze lignes, quelques dizaines de fois par saison : la dépense
n'existe pas.

**`SurveySummaryDto` ne porte pas de `statut`.** Il porte `deadline` et
`close_le` bruts ; c'est `statut_de(...)` (carte 511) qui tranche. Calculer le
statut en SQL mettrait la règle R23 dans une requête, et à deux endroits.

## Les requêtes — `sql/presences/`

| Fichier | Rôle |
|---|---|
| `find_survey_by_round.sql` | la campagne d'une journée |
| `find_answers_by_survey.sql` | ses réponses, pour la réhydratation |
| `upsert_survey.sql` | `ON CONFLICT (id) DO UPDATE` |
| `upsert_answer.sql` | `ON CONFLICT (survey_id, team_id) DO UPDATE` |
| `list_survey_summaries.sql` | **une** requête pour toute la saison |

La dette SQL de `match_day_repository` — des `INSERT` en dur dans le Rust —
n'est pas reprise. `list_survey_summaries.sql` laisse tomber la ligne de
campagne absente : une journée sans sondage doit apparaître dans la barre
latérale.

## Le dépôt reconstruit l'enum, il ne le devine pas

`presence` + `repondu_le` + `saisi_par_admin` redeviennent une `Presence`. Une
ligne incohérente — qu'aucun chemin d'écriture ne produit et que le `CHECK`
refuse — est une erreur de dépôt, `try_new(...).map_err(db_err)?`.

**Pas de `.ok()?` ici.** Le `roster_service` en porte deux, et le CLAUDE.md les
cite comme le mécanisme qui a fait disparaître un roster sans une ligne de
journal. Une réponse escamotée à la lecture, c'est une équipe qui manque au
tirage sans que personne sache pourquoi.

## Checklist

- [ ] La migration, avec ses trois `UNIQUE` et son `CHECK`
- [ ] Le port, son erreur `Database(String)`, ses DTOs de lecture
- [ ] Le dépôt, `save` transactionnel, cinq fichiers SQL
- [ ] `CompetitionsContext` reçoit `presence_survey_repository`, instancié dans `main.rs`
- [ ] Tests d'intégration (vraie `PgPool`) :
      seconde campagne sur la même journée refusée ·
      `presente` sans `repondu_le` refusée ·
      aller-retour des trois `Presence` dont `Organisateur(id)` ·
      `save` deux fois est idempotent ·
      `list_summaries` rend une ligne pour une journée sans campagne
- [ ] `make lint`, `make check-arch`, `make test`
