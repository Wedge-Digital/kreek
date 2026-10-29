# Le tirage rejoint le domaine

**Épic :** E18 — Appariement des présents · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/appariement-des-presents/calendrier/06-domaine.md` (§ 3, § 5)

## Objectif

Doter le BC `competitions` d'un algorithme de tirage juste : apparier le plus
d'équipes possible, rejouer le moins et le plus anciennement, exempter celle qui
a le plus joué, ne jamais opposer deux équipes d'un même coach, et départager au
sort. Aucun appelant à cette étape — la carte 598 le branche.

## Ce qui est repris, et comment

`src/app/competitions/domain/tirage.rs` de la branche `sondage-presence`
(commit `48d45554` et suivants, carte 507 de la branche, ses tests compris).

1. **Copie exacte** du fichier vers `domain/draw.rs` (règle 5) :
   `git show sondage-presence:src/app/competitions/domain/tirage.rs > …`.
2. **Renommage à l'outil** (LSP), sans réécriture (règle 18) :

| Branche | Ici |
|---|---|
| `tirer` | `draw` |
| `DrawInput { equipes, historique, interdites, matchs_joues }` | `DrawInput { teams, history, forbidden, matches_played }` |
| `RencontresJouees`, `Historique`, `NombreDeRencontres`, `NombreDeMatchs` | `PlayedMeetings`, `History`, `MeetingCount`, `MatchesPlayed` |
| `Recherche`, `Cout`, `Solution` | `Search`, `Cost`, `Solution` |
| `enregistrer`, `paire`, `exemptee`, `rencontres`, `optimum_prouve` | `record`, `pair`, `exempted`, `pairings`, `optimum_proven` |

Le reste des identifiants privés suit la même règle. Les commentaires restent en
français.

3. **Ce qui part** : le champ `ecartees` de `DrawProposal`, toujours vide (il ne
   servait qu'au sondage) ; la phrase « c'est la DP » du commentaire de tête,
   devenue fausse depuis que la programmation dynamique a été retirée.

## Ce qui est neuf

```rust
/// R10 — tire chaque poule ; refuse si aucune n'a deux équipes.
pub fn draw_pools(
    inputs: &[DrawInput],
    rng: &mut impl Rng,
) -> Result<Vec<DrawProposal>, DomainError>;
```

Une poule à moins de deux équipes rend une proposition vide (son équipe unique
exemptée, comme le fait déjà `draw`). `DomainError::NotEnoughPresent` quand
aucune poule n'en a deux.

## Checklist

- [x] `domain/draw.rs` copié, puis renommé à l'outil ; `domain/mod.rs`
- [x] `ecartees` retiré, commentaire de tête corrigé
- [x] `draw_pools` et `DomainError::NotEnoughPresent`
- [x] Les tests copiés passent sous leurs nouveaux noms — dont R5 sur quatre
      équipes (A-B A-C B-C jouées → deux rencontres), moindre total de
      revanches, revanche la plus ancienne, R4 au prix d'une rencontre, R8,
      l'aléa
- [x] Tests de `draw_pools` : poule à un présent sans rencontre ; aucune poule
      tirable → `NotEnoughPresent`
- [x] `generate_round_pairings` **n'est pas touché** ici (carte 598)
- [x] `make lint`, `make check-arch`, `make test`
