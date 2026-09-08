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

## R28 — trois chemins, trois autorisations

`Repondant` vaut `Jeton | Coach(CoachId) | Organisateur(CoachId)`. R19 vérifie
que l'équipe est **dans la campagne**, jamais qu'elle appartient au répondant :
sans R28, un `team_id` forgé depuis l'encart poserait une présence pour l'équipe
d'un autre coach.

Le domaine savait répondre — **`Reponse` porte déjà `coach_id`** — personne ne
lui posait la question.

`Jeton` n'est pas contrôlé, et ce n'est pas un oubli : le jeton *est*
l'autorisation (R7). Lui faire porter un `CoachId` produirait un contrôle
circulaire, comparant la réponse à elle-même.

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
| R21 | les chemins du coach — `Jeton` et `Coach` — sur une campagne close : `SurveyClosedForCoach`. L'organisateur, lui, passe |
| R28 | un `Coach(id)` dont l'identifiant ne correspond pas au `coach_id` de la réponse — `TeamNotOwnedByCoach` |
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

- [x] `EtatJournee`, `RencontreJournee`, `EffetReponse`, `Desaccord`
- [x] `enregistrer` — R19, R13, R21, R6, R12/R16
- [x] `desaccord`
- [x] `DomainError` : `TeamNotInSurvey { team }`, `SurveyClosedForCoach`,
      `RoundFrozenByReport`, `TeamNotOwnedByCoach { team }`, **plus
      `InvalidReponduLe`**
- [x] `poser_pour_test` supprimée — les six appels de 511 passent par la vraie
      méthode
- [x] Tests : les huit de la liste, plus l'exemptée qui n'est pas un orphelin et
      la journée vidée au Calendrier — 22 tests dans le fichier
- [x] `make lint`, `make check-arch`, `make test` — 1769/1769

## Ce que la réalisation a corrigé

**Une cinquième variante d'erreur, `InvalidReponduLe`.** `DateString` accepte la
chaîne vide — sa validation est `^(?:\d{4}-\d{2}-\d{2})?$` — donc
`ReponduLe::try_new(maintenant)` peut réellement échouer. C'est exactement le
motif qui a fait exister `InvalidOpenedAt` dans `ouvrir` : pas de `.expect()`
dans le domaine.

**Les deux listes de `Desaccord` sont disjointes.** Le premier test attendait que
l'adversaire d'un désistant figure dans `orphelins`. Il n'y est pas, et il ne
doit pas y être : il *est* apparié, dans une rencontre qu'il faut casser. Le
vivier à réapparier est **l'union des deux listes**, et c'est le panneau de
réparation qui la fait, sur décision de l'organisateur (carte 518). Les mêler
ferait porter à `orphelins` deux sens que rien ne distinguerait ensuite.

**R16 ne rend jamais `EnregistreeRencontreARefaire`**, et ce n'est pas une
lacune : un arrivant tardif n'apparaît dans aucune rencontre, donc aucune n'est à
refaire. `EffetReponse` dit la conséquence immédiate pour l'appelant, `desaccord`
dit l'état complet — et c'est le bénéfice de R24, puisque cet état se recalcule à
chaque affichage au lieu de se lire.
