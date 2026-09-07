# Le dépôt sait retrouver une campagne par son jeton

**Priorité : haute — la route publique ne peut rien charger sans elle**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, qui crée les tables et le port
**Fichiers :** `src/app/competitions/domain/presence_survey_repository_port.rs`,
`src/app/competitions/io/repository/presence_survey_repository.rs`,
`src/app/competitions/io/repository/sql/presences/find_survey_by_token.sql`,
`.../find_landing_labels.sql`
**Spec :** `docs/specs/sondage-presence/reponse-coach/07-integration.md`

## L'objectif

Deux requêtes de lecture, et la méthode de port délibérément reportée depuis
l'unité 1.

```rust
async fn find_by_token(&self, token: &str)
    -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

C'est un **ajout** au trait, pas une reprise : la carte 510 l'avait laissée de
côté parce que c'est cette unité qui sait ce que sa route publique charge.

## Ce que les requêtes ne font pas

**`find_survey_by_token.sql` ne consulte aucune colonne d'expiration**, parce
qu'il n'y en a pas. R7 refuse au jeton toute échéance propre : sa validité se lit
sur l'état de la campagne, calculé par `statut_de` (R23). Le jeton est un
pointeur vers une réponse, rien de plus.

**`find_landing_labels.sql` ne joint que des tables de `competitions`** —
saisons, compétitions, journées. Le nom de l'équipe vient d'`ITeamInfoPort`,
jamais d'une jointure : une jointure vers les tables de `teams` serait l'exacte
violation que la souveraineté des données entre BCs nomme.

```rust
pub struct LandingLabelsDto {   // DTO de lecture — primitives assumées
    pub round_name: String,
    pub round_dates: String,
    pub competition_id: String,
    pub competition_name: String,
    pub season_id: String,
    pub space_id: String,
}
```

## Le point qui décide d'un écran

`find_by_token` sur un jeton inconnu rend **`Ok(None)`**, jamais une erreur.
C'est cette distinction qui produit la page « lien inconnu » (R26) plutôt qu'un
`500` — et un jeton tronqué par un client mail est un cas courant, pas une panne.

## Checklist

- [ ] `find_by_token` au port et au dépôt
- [ ] `find_landing_labels`, son DTO de lecture
- [ ] Les deux fichiers SQL sous `sql/presences/`
- [ ] Tests d'intégration (vraie `PgPool`) :
      jeton connu rend la campagne **avec toutes ses réponses** ·
      jeton inconnu rend `None` et non une erreur ·
      les libellés d'une journée sans campagne restent lisibles
- [ ] `make lint`, `make check-arch`, `make test`
