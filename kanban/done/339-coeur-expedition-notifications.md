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

- [x] `use_cases/notification_dispatch.rs` : `dispatch()`, `DispatchOutcome`
- [x] `IEmailService` et `app_url` injectés dans `CompetitionsContext`
- [x] `app_url` construit depuis la configuration, schéma compris
- [x] Test `#[sqlx::test]` avec `IEmailService` **espion** : deux exécutions le
      même jour → un seul email par coach (R3)
- [x] Test : l'envoi échoue → ligne présente, `sent_at` à `NULL`, `failed = 1`
      (R1)
- [x] Test : exécution du lendemain → la ligne à `NULL` **n'est pas rejouée**
      (R9)
- [x] Test : journée décalée d'un jour → un **second** envoi part (R2)
- [x] Test : coach à deux équipes → **un** email listant **deux** matchs
- [x] `make check-arch`

## Ce qui a été fait

`app_url` **reprend l'existant** plutôt que d'ouvrir un réglage à lui : le
schéma est recollé à `host_domain`, exactement comme le fait
`send_reset_password_email`. La carte demandait de ne pas recopier ce `http://`
en dur ; l'ouvrir en second mécanisme aurait mis deux conventions dans le projet
au lieu d'une seule à réparer. La limite HTTPS est donc **partagée** avec
l'e-mail de mot de passe perdu, et une carte les corrigera ensemble.

Au passage, une fausse alerte de ma part, corrigée : le défaut de
`AppConfig::default()` vaut `"http://localhost"` — avec schéma — là où
`.env.dev` fournit `localhost:3210` sans schéma. J'y ai vu une incohérence
vivante ; elle ne l'est pas, ce défaut n'étant jamais la valeur exercée
(`EXEC_PROFILE` vaut `dev`).

Un sixième test s'ajoute aux cinq demandés : **chaque envoi ne porte qu'une
adresse**. La conception l'exige — grouper mettrait l'annuaire de l'espace dans
l'en-tête de chacun — mais aucun test de la checklist ne l'aurait vu.

## Les deux verrous qui ont mordu

**Axe 11.** L'attribut `#[tracing::instrument(...)]` réparti sur quatre lignes
se termine par `)]`, et le verrou ne lit que la ligne **immédiatement**
précédente : il refusait un use case pourtant instrumenté. L'attribut tient
désormais sur une ligne, et le commentaire dit pourquoi.

**Axe 12.** `deps.email.send(...)` était compté comme une émission d'évènement.
Déclaré par `// arch:ok`, comme le `CLAUDE.md` le prévoit pour un envoi d'e-mail.

## Vérifié en le cassant

En retirant le `return` du cas « déjà envoyé » — l'erreur qu'on ferait vraiment
— **deux** tests tombent : celui de R3 et celui de R9. Les autres passent, ce
qui montre qu'ils gardent bien autre chose.

## Relevé au passage, hors périmètre

`app::teams::io::listeners::phase_basket_purge_listener::tests::une_entree_en_ready_to_play_purge_les_deux_paniers`
est **instable** : environ 1 échec sur 10 avec la suite complète, **0 sur 15**
lancé seul. La différence désigne une course sous parallélisme, pas une faute de
logique. Aucun rapport avec cette carte — mérite la sienne.
