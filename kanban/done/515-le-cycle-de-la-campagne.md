# Le cycle de la campagne — ouvrir, relancer, clore, rouvrir

**Priorité : haute — sans elle, aucune campagne n'existe**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, 511, 513, 514
**Fichiers :** `src/app/competitions/use_cases/presences/launch_survey_use_case.rs`,
`remind_use_case.rs`, `close_survey_use_case.rs`, `reopen_survey_use_case.rs`,
`src/app/competitions/use_cases/presences/survey_mailer.rs`

## L'objectif

Quatre use cases, et le trait d'expédition que l'unité `reponse-coach`
implémentera.

## `launch_survey_use_case`

1. charge la journée — `RoundNotFound`, `IsRestDay` (R2)
2. refuse si une campagne vit déjà sur cette journée — `SurveyAlreadyOpen` (R2)
3. appelle `survey_roster_service` : destinataires et sans-adresse (R3)
4. `PresenceSurvey::ouvrir(...)` — l'agrégat engendre les réponses et les jetons
5. **persiste**
6. expédie
7. rend `LaunchOutcome { destinataires, sans_adresse }`

**L'ordre 5 avant 6 est la règle R20 :** la campagne est ouverte avant
l'expédition, et un échec d'envoi est **journalisé, pas propagé**. Un serveur de
messagerie indisponible bloquerait sinon une fonction qui reste utilisable sans
lui — l'encart du coach connecté, et la saisie manuelle de R6.

## `ISurveyMailer`, et son implémentation provisoire

```rust
pub trait ISurveyMailer: Send + Sync {
    async fn send_survey_emails(&self, survey: &PresenceSurvey, destinataires: &[...]) -> ...;
}
```

Le use case **déclare ce qu'il lui faut, il n'écrit pas d'e-mail** — c'est aussi
ce qui le rend testable sans serveur SMTP. Le gabarit, la fabrication du lien et
le journal d'envoi sont le contenu de l'unité `reponse-coach`.

**Cette carte livre une implémentation qui journalise sans envoyer**, injectée
dans `main.rs` et marquée en commentaire « remplacée en unité 2 ». C'est R20
poussée à sa conclusion : l'onglet devient **entièrement utilisable** — saisie
manuelle, tirage, validation — avant que le premier e-mail ne parte. L'attendre
aurait retardé toute vérification à l'écran jusqu'à l'unité suivante.

## Les trois autres

**`remind`** charge, liste les `SansReponse`, expédie, rend le compte. Refuse sur
campagne close — relancer pour un lien qui ne répond plus serait un e-mail qui se
contredit lui-même.

**`close`** et **`reopen`** sont **deux fichiers, pas un avec un booléen** : le
workflow interdit le use case fourre-tout, et les deux n'ont ni les mêmes gardes
ni les mêmes conséquences — rouvrir réarme des jetons, clore n'en réarme aucun.
`reopen` reçoit une nouvelle échéance (R23) et refuse une journée figée (R13).

## Conventions

`#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` sur les quatre.
`skip_all` est obligatoire — sans lui l'attribut tente d'enregistrer les dépôts,
qui n'implémentent pas `Debug`. Aucune commande ne porte de secret : le jeton est
engendré par l'agrégat, il n'entre jamais par une commande.

**Aucun domain event.** Personne hors du BC n'a à savoir qu'un sondage est
ouvert, et R14 le garantit.

## Checklist

- [x] Les quatre commandes, avec leurs value objects (jamais de primitive nue)
- [x] Les quatre use cases, chacun avec son enum d'erreur et ses `From<...>`
- [x] `ISurveyMailer` + l'implémentation qui journalise, au contexte du BC
- [x] R20 : l'échec d'envoi est journalisé et n'échoue pas le lancement
- [x] Tests unitaires : 15 sur les quatre use cases, doubles partagés
- [x] `make lint`, `make check-arch`, `make test` — 1816/1816

## Ce que la réalisation a tranché

**Le trait ne rend pas de `Result`.** R20 dit qu'un échec d'envoi est journalisé
et non propagé ; avec un `Result`, tenir la règle dépendrait de la discipline de
chaque appelant — un `?` de trop et la panne du serveur SMTP annule le lancement.
Avec un `RapportEnvoi`, **il n'y a rien à propager** : la règle devient
structurelle au lieu d'être une consigne. Même raisonnement que « pas
d'implémentation par défaut qui bouclerait sur `save_pairing` ».

**L'implémentation provisoire rend `envoyes: 0`, pas `envoyes: n`.** Rien n'est
parti, et le prétendre afficherait « 14 e-mails envoyés » à l'organisateur. R20
accepte qu'un envoi échoue ; elle n'autorise pas à mentir sur son issue. Elle ne
journalise ni les adresses ni les jetons — donnée personnelle d'un côté, secret
d'URL de l'autre (R7).

**La journée de repos est refusée par le domaine, pas par le use case.** La carte
mettait `IsRestDay` à l'étape 1, mais `ouvrir` refuse déjà avec
`SurveyOnRestDay` depuis la 511. Le use case relaie au lieu de rejuger — comme
`update_pools_settings` relaie `InvalidPools`. La garder aux deux étages ferait
dire R2 deux fois.

**`SurveyAlreadyOpen` est devenue `SurveyAlreadyExists`.** Ce que la garde
vérifie est l'existence, pas l'ouverture : une campagne close occupe la journée
tout autant, et se **rouvre** au lieu de se relancer. Le nom d'origine ferait
croire qu'une campagne close peut être relancée, ce qui perdrait ses réponses.

**Un module de doubles partagé**, `presences/test_doubles.rs`.
`IMatchDayRepository` a treize méthodes dont deux servent ici ; les recopier
quatre fois — pour 515 à 518 — ferait de leur signature une dette répartie qu'un
ajout au port obligerait à réparer partout. Les cinq faux existants du BC ne sont
pas repris : ils sont internes à leur module de test, et les partager serait une
refonte sans rapport avec cette carte.

**`etat_de_la_journee` est publique**, et c'est le patron que 516 et 517
reprendront : un seul aller-retour au port pour tous les appariements de la
journée, là où un appel par rencontre en ferait quinze pour une réponse
booléenne. L'axe 11 l'a attrapée — `pub async fn` dans `use_cases/` sans
instrumentation — et elle porte son `// arch:no-instrument` avec son motif.
