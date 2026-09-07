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

- [ ] `valider_proposition` — R5, R10, R18, R22
- [ ] `clore`, `rouvrir` — R13, R23
- [ ] `marquer_appariee`, `defaire_appariement` — R9
- [ ] `DomainError` : `ForbiddenPair`, `TeamNotPresent`, `TeamNoLongerEnrolled`,
      `InconsistentProposal { motif }`
- [ ] Tests : R22 (équipe absente refusée, paire interdite refusée, équipe en
      double refusée) · R23 (`rouvrir` avec une échéance passée refusé) ·
      R13 (`rouvrir` sur journée figée refusé) ·
      R9 (l'exemptée qui reprend du service n'est pas comptée exemptée)
- [ ] `make lint`, `make check-arch`, `make test`
