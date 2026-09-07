# La page de réponse sait ce qu'elle a à dire

**Priorité : moyenne — sans elle, la page nomme des identifiants**
**Épic :** E16 — Sondage de présence
**Dépend de :** 523
**Fichiers :** `src/app/competitions/use_cases/presences/presence_landing_service.rs`

## Le trou que cette carte comble

La page affiche « Journée 3 · du 12 au 19 octobre », « Ton équipe : Les Rats
d'Égouts », et un lien vers la compétition. **`PresenceSurvey` ne porte aucun de
ces libellés** : il a des identifiants, une échéance et des réponses.

Trouvé en phase 4, et il ne se comble pas par des ports appelés depuis le
handler — ce serait exposer `TeamInfoDto` à la couche web.

## Conception

```rust
// arch:no-instrument — service d'hydratation : assemble une vue, sans intention métier
pub async fn hydrater(
    survey: &PresenceSurvey,
    token: &SurveyToken,
    survey_repo: &dyn IPresenceSurveyRepository,
    team_port: &dyn ITeamInfoPort,
) -> Option<LandingContext>

pub struct LandingContext {
    pub team_name: String,
    pub round_label: String,
    pub competition_name: String,
    pub competition_url: String,
    pub deadline: String,
    pub presence: Presence,      // du domaine, pas une chaîne
    pub statut: SurveyStatus,    // du domaine
}
```

**`presence` et `statut` traversent en types du domaine.** C'est le VM, en bout
de chaîne, qui choisit les mots — pas ce service. Les aplatir ici en `String`
aurait mis la formulation dans la couche applicative, et rendu le VM incapable de
distinguer un état d'un libellé.

**`find_team_names(&[String])` existe déjà** sur `ITeamInfoPort`, à côté de
`find_enrolled_teams`. Aucune méthode de port à ajouter.

## Checklist

- [ ] `LandingContext`, `hydrater`, le marqueur `arch:no-instrument` avec son motif
- [ ] `TeamInfoDto` et `LandingLabelsDto` ne sortent pas du service
- [ ] Tests unitaires avec dépôt et port simulés : une équipe dont le nom
      manque au port · une campagne close · une réponse `SansReponse`
- [ ] `make lint`, `make check-arch`, `make test`
