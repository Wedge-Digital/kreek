# Écrire un tirage est atomique

**Priorité : moyenne — une panne au milieu laisse une journée à moitié appariée**
**Épic :** E16 — Sondage de présence
**Dépend de :** rien
**Fichiers :** `src/app/competitions/domain/match_day_repository_port.rs`,
`src/app/competitions/io/repository/match_day_repository.rs`,
`src/app/competitions/use_cases/admin/generate_pairings.rs`,
et les cinq `FakeMatchDayRepo`

## Le défaut

Générer les appariements d'une journée écrit N paires et N projections, **une
par une**. Une panne au troisième laisse une journée à moitié appariée, et rien
ne le signale.

Pire : deux organisateurs qui régénèrent en même temps franchissent tous les
deux la garde « cette journée porte-t-elle déjà des appariements ? » avant que
l'un ait écrit. La garde est juste, elle est simplement lue trop tôt.

## Conception

```rust
async fn save_pairings(
    &self,
    match_day_id: &str,
    pairings: &[(Pairing, NewPairingProjection)],
) -> Result<(), MatchDayRepositoryError>;
```

Transaction, `SELECT ... FOR UPDATE` sur la journée, N paires et N projections,
commit.

**Le verrou seul ne suffisait pas** — précisé à l'implémentation. La garde de
R11 est lue dans `execute`, *avant* la transaction : le second organisateur
attendrait le verrou, l'obtiendrait, et écrirait quand même ses doublons. Le
verrou ne serait alors qu'un ralentisseur, sérialisant deux écritures fautives
au lieu de les paralléliser.

`save_pairings` **revérifie sous le verrou** que la journée est vide, et rend
`MatchDayRepositoryError::PairingsAlreadyExist` sinon. C'est cette relecture qui
fait tenir R11, le verrou ne faisant que la rendre fiable.

**Le verrou porte sur la journée, pas sur ses appariements** : il n'y a rien à
verrouiller dans une table vide, et c'est justement le cas qui nous intéresse.

**La transaction couvre la journée entière, pas la poule.** Une transaction par
poule laisserait encore une journée à moitié appariée si la troisième échouait.
`generate_pairings` collecte donc les appariements de toutes ses poules, écrit
en un appel, **puis** émet ses `PairingCreated` — après le commit, jamais
dedans : un listener qui réagit à un événement dont la transaction est ensuite
annulée aurait travaillé sur un fait qui n'a pas eu lieu.

**`save_pairing` n'est pas modifiée.** Elle a trois appelants réels, dont deux
écrivent un seul appariement et n'ont rien à gagner à une transaction. Seul
`generate_pairings` migre : il écrit N appariements et souffre du même défaut.

**Pas de méthode par défaut** sur le trait qui boucle sur `save_pairing`. Elle
compilerait, les cinq fakes n'auraient rien à changer, et le vrai dépôt
resterait non atomique le jour où quelqu'un oublierait de la redéfinir — un
verrou qui se laisse oublier n'est pas un verrou.

## Le coût, mesuré

Cinq fakes implémentent `IMatchDayRepository` :
`match_report_published_listener`, `delete_pairing_use_case`,
`generate_pairings`, `add_match_use_case`, `generate_all_pairings`. Quatre
lignes chacun.

## Checklist

- [ ] `save_pairings` au port, sans implémentation par défaut
- [ ] Implémentation : transaction, `FOR UPDATE`, paires et projections ensemble
- [ ] Les cinq fakes l'implémentent
- [ ] `generate_pairings` l'appelle
- [ ] `ecrire_appariement` extraite du corps de `save_pairing` — les deux
      méthodes partagent l'écriture, deux copies du même `INSERT` divergeraient
      à la première colonne ajoutée
- [ ] Test unitaire : un dépôt en échec n'écrit rien **et n'émet rien**
- [ ] Tests d'intégration sur vraie base : les N appariements et leurs N
      projections ensemble · une seconde génération refusée · zéro appariement
      ne verrouille rien
- [ ] `make lint`, `make check-arch`, `make test`
