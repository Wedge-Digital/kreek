# Ajouter un membre, côté applicatif

**Priorité : haute**
**Dépend de :** 376 et 377
**Conception :** `docs/specs/space-admin/ajout-direct/05-use-cases.md`
**Fichiers :** `src/app/spaces/use_cases/add_member_use_case.rs`, `context.rs`

## Objectif

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: AddMemberCommand, repo: &dyn ISpaceRepository,
                     email: &dyn IEmailService, bus: &EventBus)
    -> Result<MembershipOutcome, AddMemberError>;
```

Même forme que les deux use cases de la carte 367 : charger, appeler, persister,
notifier, émettre, rendre le compte.

Le compte d'administrateurs sert ici aussi : ajouter un administrateur peut
faire passer l'espace de un à deux, ce qui **dégèle** la ligne du premier dans
l'onglet Membres.

## La notification est envoyée par le use case

`notifier` est une case à cocher — l'état d'une interface au moment d'un clic.
La faire voyager dans l'événement domaine l'inscrirait pour toujours au journal,
et un lecteur futur y verrait une propriété du fait « ce coach a été ajouté »,
qu'elle n'est pas.

Le use case lit le drapeau de la commande et appelle `IEmailService`. C'est le
précédent d'`auth`, dont `send_reset_password_email` envoie directement.

**`SpacesContext` gagne `email_service: Arc<dyn IEmailService>`**, comme
`AuthContext`. `crate::common::services::email` ne figure pas dans ce qu'un BC
extractible s'interdit : service de la couche commune, ni autre BC ni hôte.

## Un échec d'envoi ne fait pas échouer l'ajout

L'appartenance est posée, l'événement émis, et un email qui ne part pas est
journalisé en `warn`.

Refuser l'ajout parce que le serveur de mail est indisponible ferait dépendre
une règle d'appartenance d'un service qui n'en gouverne aucune. C'est l'inverse
du choix fait dans la carte 379, où l'email **est** l'accès — et les deux sont
cohérents.

## Checklist

- [ ] Use case, `AddMemberCommand` sans primitive nue — `Notification` est un
      value object, pas un `bool`
- [ ] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`, `skip_all`
      obligatoire
- [ ] `SpacesContext` gagne `email_service`, câblé dans `main.rs`
- [ ] Émission par `emettre()`, jamais `.send(`
- [ ] Gabarit d'email de courtoisie, sous `emails/fr_FR/`
- [ ] Tests unitaires sur `FakeRepo` et service d'email factice :
  - [ ] ajout nominal en Membre → compte inchangé ; en Admin → compte +1
  - [ ] coach déjà membre → `Metier(DejaMembre)`, **aucune écriture, aucun
        événement, aucun email**
  - [ ] `Notification::Envoyer` → un envoi ; `Taire` → aucun
  - [ ] **l'envoi échoue → `Ok` quand même, et l'écriture a eu lieu** — le seul
        test qui vérifie que la courtoisie ne gouverne pas l'appartenance
- [ ] `make lint`, `make check-arch`, `make test` passent
