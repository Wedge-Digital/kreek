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

- [x] Use case, `AddMemberCommand` sans primitive nue — `Notification` est un
      value object, pas un `bool`
- [x] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`
- [x] `SpacesContext` gagne `email_service`, câblé dans `main.rs`
- [x] Émission par `emettre()`, jamais `.send(` — et l'envoi d'e-mail déclaré
      par `// arch:ok`, que l'axe 12 exige **sur la ligne immédiatement
      précédente**
- [x] Gabarit d'email de courtoisie, sous `emails/fr_FR/`
- [x] Sept tests unitaires sur `FakeSpaceRepo`, un cache factice et un service
      d'email factice :
  - [x] ajout nominal en Membre → compte inchangé ; en Admin → compte +1
  - [x] coach déjà membre → `Metier(DejaMembre)`, **aucune écriture, aucun
        événement, aucun email**
  - [x] `Notification::Envoyer` → un envoi ; `Taire` → aucun
  - [x] **l'envoi échoue → `Ok` quand même, écriture et événement bien là**
  - [x] **ajouté** : un coach absent du cache est refusé
- [x] Le test de l'envoi qui échoue **vu échouer** sur un envoi rendu bloquant
- [x] `make lint`, `make check-arch`, `make test` passent — 1161 tests

## Ce qu'on a appris en la faisant

**Le use case a besoin de deux dépôts.** `add_member` prend un `Coach` complet
depuis la carte 376, donc le cache d'utilisateurs entre en jeu. D'où une erreur
de plus, `CoachInconnu`, et son test : sans pseudo, il n'y a pas de `Coach` à
construire.

**L'axe 12 a fait son travail sur un `.send(` qui n'en était pas un.** C'est le
cas que la règle prévoit — envoi d'e-mail, requête HTTP — et le marqueur
`// arch:ok` doit être **sur la ligne immédiatement précédente**, pas deux lignes
au-dessus. Le verrou est plus strict que sa description ne le laissait croire.

**Deux classes CSS inventées dans le gabarit d'email.** `cta` et `muted`
n'existent pas dans sa feuille — elles n'auraient simplement rien fait. Les
classes réellement définies ont été relevées et vérifiées une par une.
