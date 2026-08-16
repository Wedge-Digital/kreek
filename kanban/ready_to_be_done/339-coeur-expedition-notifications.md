# Le cœur d'expédition

**Spec :** `docs/specs/notifications/envoi/05-use-cases.md`
**Dépend de :** 335, 337, 338
**Ouvre :** 340

## Objectif

Ce que les deux déclencheurs partagent : réserver, rendre, envoyer, confirmer.

## Conception

```rust
pub async fn dispatch(
    notification: NotificationType,
    season:       &SeasonContext,
    round:        Option<&RoundRef>,
    target_date:  &DateString,
    deps:         &DispatchDeps<'_>,
) -> DispatchOutcome;
```

Pour chaque destinataire, dans cet ordre :

1. **Réserver** la ligne du journal. Zéro ligne insérée → déjà envoyé, on passe.
   C'est la base qui tranche, pas le code.
2. **Rendre** le gabarit avec le contexte du destinataire.
3. **Envoyer**, un destinataire à la fois.
4. **Confirmer**, ou laisser la ligne à `NULL`.

### Un envoi par destinataire, jamais groupé

`IEmailService::send` prend un `Vec<String>` de destinataires, ce qui invite à
expédier trente coachs en un appel. **Ce serait mettre les trente adresses en
clair dans l'en-tête que chacun reçoit** — un annuaire de l'espace distribué à
tout le monde. R4 impose de toute façon un corps personnalisé.

### Aucune transaction n'enveloppe les quatre étapes

C'est délibéré. Un appel réseau se produit entre la réservation et la
confirmation, et tenir une transaction ouverte pendant un aller-retour HTTP est
précisément ce que les garde-fous de la carte 317 cherchaient à empêcher. Ne pas
« corriger » cela par zèle.

### Un échec unitaire n'interrompt pas la boucle

Le destinataire est compté en `failed`, sa ligne reste à `NULL`, on continue. Une
adresse invalide ne doit pas priver les vingt-neuf autres de leur email.

## Checklist

- [ ] `use_cases/notification_dispatch.rs` : `dispatch()`, `DispatchOutcome`
- [ ] `IEmailService` et `app_url` injectés dans `CompetitionsContext`
- [ ] `app_url` construit depuis la configuration, schéma compris
- [ ] Test `#[sqlx::test]` avec `IEmailService` **espion** : deux exécutions le
      même jour → un seul email par coach (R3)
- [ ] Test : l'envoi échoue → ligne présente, `sent_at` à `NULL`, `failed = 1`
      (R1)
- [ ] Test : exécution du lendemain → la ligne à `NULL` **n'est pas rejouée**
      (R9)
- [ ] Test : journée décalée d'un jour → un **second** envoi part (R2)
- [ ] Test : coach à deux équipes → **un** email listant **deux** matchs
- [ ] `make check-arch`
