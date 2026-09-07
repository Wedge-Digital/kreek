# L'encart sait quelles campagnes sont ouvertes

**Priorité : moyenne — l'encart ne peut rien afficher sans elle**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, qui crée les tables et le port
**Fichiers :** `src/app/competitions/domain/presence_survey_repository_port.rs`,
`src/app/competitions/io/repository/presence_survey_repository.rs`,
`.../sql/presences/list_open_surveys_for_season.sql`
**Spec :** `docs/specs/sondage-presence/encart-competition/07-integration.md`

## L'objectif

```rust
async fn list_open_surveys_for_season(&self, season_id: &str, maintenant: &str)
    -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

Une requête pour la saison, jamais une par journée — l'encart doit connaître
toutes les campagnes ouvertes, et les chercher une à une ferait vingt
allers-retours à chaque affichage de la page de détail.

## Elle rend des agrégats, pas un DTO de lecture

L'encart a besoin de `statut()`, de la réponse de chaque équipe et de son
horodatage : trois questions du domaine. Un DTO les aurait aplaties, et le VM
aurait dû les reconstituer — exactement ce que « un view model transpose, il ne
dérive pas » interdit.

## Le filtre vit à deux endroits, et c'est assumé

Le SQL **élague** — échéance passée, `close_le` renseigné — puis `statut()`
**décide**. R23 fait de la clôture un calcul, donc la règle est en principe au
domaine seul.

L'alternative serait de charger toutes les campagnes de la saison pour en jeter
la plupart, à chaque affichage d'une page que la majorité des visiteurs ouvre
sans être concernée.

**Ce qui rend le compromis sûr** : la référence reste le domaine, et un désaccord
entre les deux ne produit qu'une campagne chargée pour rien — **jamais une
campagne affichée à tort**. L'asymétrie est ce qui autorise l'élagage.

## Checklist

- [ ] La méthode au port et au dépôt
- [ ] `list_open_surveys_for_season.sql`, sous `sql/presences/`
- [ ] Tests d'intégration (vraie `PgPool`) :
      une campagne échue de la veille n'est pas rendue ·
      une campagne close par décision non plus, **échéance à venir** ·
      une campagne ouverte revient avec **toutes** ses réponses ·
      deux campagnes ouvertes reviennent toutes les deux
- [ ] `make lint`, `make check-arch`, `make test`
