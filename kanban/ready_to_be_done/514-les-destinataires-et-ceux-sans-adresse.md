# Les destinataires, et ceux sans adresse

**Priorité : moyenne — sans lui, les DTOs de port remonteraient jusqu'aux gabarits**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, 511
**Fichiers :** `src/app/competitions/use_cases/presences/survey_roster_service.rs`

## L'objectif

Croiser les équipes engagées, les adresses des coachs et les réponses de la
campagne, et rendre des **objets du domaine local**.

C'est le cas d'école de la section « Domain services pour données inter-BCs » du
CLAUDE.md : `TeamInfoDto` et `SpaceMemberDto` **n'atteignent jamais un handler
ni un gabarit**.

## Conception

| Besoin | Port | Ce qu'il rend |
|---|---|---|
| les équipes engagées de la saison | `ITeamInfoPort::find_enrolled_teams` | `team_id`, `team_name`, `coach_id`, `coach_name`, `logo_url` |
| l'adresse des coachs | `ICompetitionSpaceMemberPort::list_space_members` | `coach_id`, `coach_name`, `email` |

**Aucun port à créer, aucun adapter à écrire** — les deux existent et sont déjà
dans le contexte.

`TeamInfoDto` ne porte pas l'adresse : c'est le croisement des deux listes sur
`coach_id` qui produit les destinataires, et **son défaut de correspondance qui
produit le compte « sans adresse connue » de R3**. Un coach sans adresse
n'empêche pas le lancement — son équipe entre d'emblée dans la colonne « sans
réponse », avec sa mention. Refuser le lancement ferait dépendre une campagne de
quatorze coachs de la fiche incomplète d'un seul.

**`coach_label` est construit ici**, pas dans le gabarit : « Lepandawan · 2
équipes » suppose de savoir combien d'équipes ce coach engage dans cette saison,
ce qui est une question sur le roster de la campagne, pas sur la ligne affichée.

## Checklist

- [ ] Le croisement des deux ports sur `coach_id`
- [ ] Le compte des sans-adresse (R3)
- [ ] `coach_label` avec le nombre d'équipes du coach
- [ ] La composition avec les réponses de la campagne, pour les trois colonnes
- [ ] `// arch:no-instrument` si la fonction est async et n'est pas un use case
      — service d'hydratation, sans intention métier
- [ ] Tests unitaires : un coach à deux équipes, un coach sans adresse, une
      équipe dont le coach n'est plus membre de l'espace
- [ ] `make lint`, `make check-arch`, `make test`
