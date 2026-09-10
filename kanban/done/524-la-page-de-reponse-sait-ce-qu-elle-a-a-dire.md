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

- [x] `LandingContext`, `hydrater`, le marqueur `arch:no-instrument` avec son motif
- [x] `TeamInfoDto` et `LandingLabelsDto` ne sortent pas du service
- [x] Sept tests unitaires, dépôt et port simulés
- [x] `make lint`, `make check-arch`, `make test` — 1890/1890

## Deux écarts au `LandingContext`, pour la même raison

**`competition_url` disparaît.** Le construire obligerait un service de
`use_cases/` à connaître `AppRoutes`, qui est de la couche web — et
`notification_recipients.rs` a écarté un `match_url` pour ce motif exact, avec le
commentaire qui l'explique. Le service rend `competition_id`, `season_id` et
`space_id` ; le gabarit compose.

**`round_label` devient `round_name` et deux dates brutes.** « Journée 3 · du 12
au 19 octobre » se compose déjà dans la couche web par `dates_de` ; le refaire ici
le mettrait à deux endroits. Même raison qu'en carte 523, et même conséquence.

Et `deadline` sort en `SurveyDeadline` plutôt qu'en `String` : la carte laissait
ce champ aplati, alors que le type existe et que c'est la vue qui formate — la
même règle que pour `presence` et `statut`.

## Une seule sortie vide, et c'est R26

Jeton étranger, libellés manquants, équipe introuvable : tout rend `None`, et la
route affichera la même page « lien inconnu ». Deux sorties distinctes révéleraient
qu'un jeton a existé, ce que R26 refuse. Un test compare les deux cas
explicitement.

`aujourd_hui` est une **entrée** — la carte l'avait omis. Le statut se calcule
(R23), et un service qui lit l'horloge n'est testable qu'en trichant sur celle de
la machine.

## Un double qui ne prouvait rien

`FauxTeams::find_team_names` rendait une **liste vide en toutes circonstances**.
Ça suffisait aux use cases qui ne l'appellent pas ; cette carte est la première à
s'en servir, et son test a échoué sur « Ton équipe » au lieu du vrai nom.

Le double se comporte désormais comme le vrai port : il rend les équipes connues
et omet les identifiants introuvables. **Un double qui ne répond jamais ne prouve
rien** — celui-là aurait laissé passer n'importe quelle erreur de résolution.

`destinataires(n)` est remonté dans les doubles partagés plutôt que recopié : les
cartes 525 à 528 en auront besoin.
