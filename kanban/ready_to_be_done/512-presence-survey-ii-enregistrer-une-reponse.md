# `PresenceSurvey` II — enregistrer une réponse

**Priorité : haute — c'est le seul chemin d'écriture d'une présence**
**Épic :** E16 — Sondage de présence
**Dépend de :** 511, qui pose l'agrégat
**Fichiers :** `src/app/competitions/domain/presence_survey.rs`,
`src/app/competitions/domain/error.rs`

## L'objectif

`enregistrer` et `desaccord` — les deux méthodes qui portent cinq règles à elles
seules, et que les trois unités appellent : l'organisateur depuis les boutons de
la carte, le coach depuis son jeton, le coach connecté depuis l'encart.

## `enregistrer`

```rust
enregistrer(&mut self, team, venue, par, journee: &EtatJournee, maintenant)
    -> Result<EffetReponse, DomainError>

pub struct EtatJournee {
    pub figee:      bool,                    // R13 — un rapport publié
    pub rencontres: Vec<RencontreJournee>,   // (pairing_id, home, away) — R24
}

pub enum EffetReponse {
    Enregistree,
    EnregistreeRencontreARefaire { pairing: PairingId },
}
```

**Des faits, pas des ports.** Le use case interroge `IMatchReportStatusPort` et
le dépôt de journées une fois chacun et passe le résultat ; l'agrégat décide. La
question « est-ce autorisé ? » reste dans le domaine.

**Les rencontres entières, pas la seule rencontre touchée.** Un paramètre
`rencontre_de: Option<PairingId>` calculé par le use case aurait sorti du
domaine la question « quelle rencontre est touchée ? », qui est métier.

Les règles portées :

| Règle | Ce que la méthode refuse ou fait |
|---|---|
| R19 | un `TeamId` qui n'est pas dans la campagne — `TeamNotInSurvey` |
| R13 | une journée figée par un rapport publié — `RoundFrozenByReport` |
| R21 | le chemin du **coach** sur une campagne close — `SurveyClosedForCoach`. L'organisateur, lui, passe |
| R6 | `Repondant::Organisateur(id)` est conservé dans `Presence::Declaree` |
| R12/R16 | si la journée est appariée et que la rencontre de l'équipe est touchée, rend `EnregistreeRencontreARefaire` |

**Elle signale, elle ne répare pas.** La maquette montre une proposition que
l'organisateur valide, jamais un fait accompli. Un `Result<(), _>` aurait obligé
le use case à redécouvrir tout seul qu'une rencontre est touchée.

## `desaccord`

```rust
desaccord(&self, journee: &EtatJournee) -> Option<Desaccord>
```

Croise les présences avec les appariements **réels** de la journée et rend ce qui
ne concorde pas : un présent que rien n'apparie, un apparié devenu absent.

**Elle se recalcule, elle ne se lit pas** — c'est R24. L'agrégat ne possède plus
les rencontres, donc l'état « défection à traiter » ne peut plus diverger de la
réalité, et il **survit à un rechargement de page**. La forme précédente tenait à
ce que l'agrégat et la journée restent d'accord, ce que quatre chemins du
Calendrier rendaient impossible.

## Checklist

- [ ] `EtatJournee`, `RencontreJournee`, `Desaccord`
- [ ] `enregistrer` — R19, R13, R21, R6, R12/R16
- [ ] `desaccord`
- [ ] `DomainError` : `TeamNotInSurvey { team }`, `SurveyClosedForCoach`,
      `RoundFrozenByReport`
- [ ] Tests : R6 (l'identifiant de l'organisateur est conservé) ·
      R12 (présent -> absent après appariement) · R16 (absent -> présent aussi) ·
      R13 (journée figée refusée) · R19 (équipe hors campagne) ·
      R21 (close : `Coach` refusé, `Organisateur` accepté) ·
      R24 (`desaccord` sur une journée vidée au Calendrier)
- [ ] `make lint`, `make check-arch`, `make test`
