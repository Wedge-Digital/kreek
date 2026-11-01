# Le camp du coach courant

**Épic :** E19 — Saisie du compte rendu : droits et robustesse · **Ordre :** 1 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E19-cadrage.md` (R3)

## Objectif

Le compte rendu sait dire **qui est ce coach dans ce match** : domicile,
visiteur, les deux — il coache les deux équipes —, ou aucun. Aucun changement
visible : c'est la question que la garde des routes (622) pose, et que E22
posera à chaque étape pour restreindre un coach à son camp.

## Conception

Dans `use_cases/match_report_access_service.rs`, à côté de `is_authorized` :

```rust
pub enum CampDuCoach { Domicile, Visiteur, LesDeux, Aucun }

// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn camp_du_coach(deps: &AccesRapportDeps<'_>, scope: &PorteeRapport, user_id: &str)
    -> CampDuCoach;
```

Le port `ITeamDataPort::is_coach_of_team` existe ; aucun port nouveau. Une
erreur de port vaut « pas coach » de cette équipe — le contrôle échoue fermé.
`is_coach_of_either_team` devient `camp_du_coach(...) != Aucun` : un seul
calcul, deux usages.

L'admin n'a pas de camp : `is_authorized` continue de le traiter à part.

## Checklist

- [ ] `CampDuCoach`, `camp_du_coach`, et `is_authorized` qui s'appuie dessus
- [ ] Tests unitaires sur une doublure du port : domicile, visiteur, les deux,
      aucun, port en erreur sur une équipe
- [ ] `make lint`, `make check-arch`, `make test`
