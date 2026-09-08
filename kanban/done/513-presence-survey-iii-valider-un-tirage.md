# `PresenceSurvey` III — valider un tirage

**Priorité : haute — l'agrégat garde le dernier mot sur ce qui s'écrit**
**Épic :** E16 — Sondage de présence
**Dépend de :** 511, qui pose l'agrégat
**Fichiers :** `src/app/competitions/domain/presence_survey.rs`,
`src/app/competitions/domain/error.rs`

## L'objectif

Les quatre dernières méthodes de commande : `valider_proposition`, `clore`,
`rouvrir`, `marquer_appariee` / `defaire_appariement`.

## `valider_proposition` — R22

```rust
valider_proposition(&self, prop, engagees, interdites) -> Result<(), DomainError>
```

L'aperçu du tirage **ne persiste rien** : la proposition voyage par le client,
donc elle revient modifiable. Elle est revalidée intégralement :

| Vérification | Règle | Erreur |
|---|---|---|
| toutes les équipes sont présentes | R5 | `TeamNotPresent { team }` |
| toutes sont encore engagées | R18 | `TeamNoLongerEnrolled { team }` |
| aucune paire interdite | R10 | `ForbiddenPair { home, away }` |
| aucune équipe en double, exemptée cohérente avec la parité | R22 | `InconsistentProposal { motif }` |

**Ce n'est pas de la défiance envers l'organisateur.** L'état a pu changer entre
l'aperçu et la validation : une équipe désengagée, une réponse modifiée dans un
autre onglet. La proposition était juste quand elle a été calculée, et ne l'est
plus.

`motif` est un `&'static str` et non un `String`, comme `ImmutableTierField` du
même enum : il ne peut venir que du code qui a détecté l'écart, jamais d'une
requête.

## `clore` et `rouvrir` — R23, R13

`clore(maintenant)` pose `Fermeture::Decidee`. `rouvrir(nouvelle_deadline,
figee, maintenant)` **exige une nouvelle échéance** : la clôture étant calculée,
rouvrir sans repousser la date rouvrirait sur une campagne close dans la
seconde. Il refuse une échéance déjà passée, pour la même raison, et une journée
figée par un rapport publié — rouvrir réarme les jetons (R7), donc rouvrir une
journée jouée rouvrirait la porte à des réponses sur un fait accompli.

Rouvrir une journée **appariée mais non jouée** reste permis : c'est le chemin
normal quand une défection arrive après le tirage.

## `marquer_appariee` — R9

```rust
marquer_appariee(&mut self, exemptee: Option<TeamId>)
defaire_appariement(&mut self)
```

**Elle enregistre l'exemption qui a eu lieu, pas celle qui était proposée.** La
nuance vient de R9 croisée à R12 : quand l'exemptée reprend du service après une
défection, elle n'a finalement pas été exemptée, et la compter comme telle la
ferait passer devant à la journée suivante pour une exemption qu'elle n'a pas
subie. C'est aussi pourquoi la réparation la rappelle avec la nouvelle exemptée,
au lieu d'une méthode `remplacer_rencontre` distincte : les rencontres ayant
quitté l'agrégat (R24), il n'y resterait rien à remplacer.

## Checklist

- [x] `valider_proposition` — R5, R10, R18, R22
- [x] `clore`, `rouvrir` — R13, R23
- [x] `marquer_appariee`, `defaire_appariement` — R9
- [x] `DomainError` : les quatre prévues, **plus `DeadlineInThePast` et
      `InvalidFermeeLe`**
- [x] `peut_tirer` reçoit `inscrites` — hors périmètre initial, cf. ci-dessous
- [x] Tests : les quatre de la liste, plus la parité dans ses deux sens, la
      désinscrite qui ne condamne pas l'exemption, `clore` idempotente,
      l'échéance du jour même, la journée appariée non jouée, et la désinscrite
      qui ne compte pas comme appariable — **40 tests dans le fichier**
- [x] `make lint`, `make check-arch`, `make test` — 1787/1787

## Ce que la réalisation a corrigé

**La parité, écrite pour survivre à R10.** La carte disait « exemptée cohérente
avec la parité » sans la définir. Le critère strict — pair ⇒ pas d'exemptée —
refuserait une proposition légitime : quatre présents dont trois équipes d'un même
coach donnent une rencontre, une exemptée et une équipe qui rentre sans jouer,
soit un effectif **pair avec une exemptée**. `Cout::perdues` existe précisément
pour ce cas. La forme retenue — *une exemptée n'est justifiée que si elle n'avait
personne à jouer* — se réduit exactement au critère strict dès que R10 ne mord
pas. Deux tests l'encadrent.

**Une désinscrite laissée sans match ne condamne pas l'exemption.** Cas non
prévu, apparu en écrivant `verifier_l_exemption` : R18 l'a déjà écartée du
tirage, elle n'attend pas d'adversaire. Sans cette exclusion, une seule
désinscription rendait toute exemption incohérente.

**`peut_tirer` ne connaissait pas l'inscription — et la garde R15 était donc
inopérante.** Elle comptait `compte_presents()`, or une équipe désinscrite ne
participe pas au tirage : deux présents dont une désinscrite activaient le
bouton, l'aperçu revenait vide, et l'organisateur tombait exactement sur le
symptôme que R15 existe pour éviter. Elle reçoit désormais `inscrites`, comme
`valider_proposition`, et `compte_appariables` est exposée pour que l'écran de la
519 cite le même nombre que la garde plutôt que de le recompter.

**Deux variantes d'erreur en plus.** `DeadlineInThePast` porte la règle que la
carte décrivait en toutes lettres — « il refuse une échéance déjà passée » — mais
avait omise de sa liste. `InvalidFermeeLe` complète la série `InvalidOpenedAt` /
`InvalidReponduLe` : `DateString` accepte la chaîne vide, donc `FermeeLe::try_new`
peut échouer, et le domaine ne panique pas.

**`rouvrir` reçoit `&EtatJournee` et non `figee: bool`** — la carte a été écrite
avant que la 512 n'invente le type. Une seule forme pour « les faits de la
journée ».
