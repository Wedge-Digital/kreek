# Page hôte + onglet Membres — Use cases

**Entrée :** `04-dtos.md` validé.

Deux mutations, donc deux use cases. Les widgets sont en lecture et n'en ont
pas : ils lisent le dépôt et construisent leurs VMs.

## Ce qu'ils rendent, et pourquoi

Les deux rendent le **nombre d'administrateurs postérieur** à l'opération.

Le contrôleur doit re-rendre la ligne modifiée, et `role_locked` dépend de ce
nombre : rétrograder l'avant-dernier administrateur fige le `select` du dernier.
Sans cette valeur, le contrôleur relirait toute la liste juste pour compter —
une seconde lecture pour une donnée que le use case tient déjà en main, l'agrégat
étant chargé.

```rust
pub struct MembershipOutcome {
    pub administrateurs: usize,
}
```

## `change_member_role_use_case`

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ChangeMemberRoleCommand,
    repo: &dyn ISpaceRepository,
    bus: &EventBus,
) -> Result<MembershipOutcome, ChangeMemberRoleError>;
```

**Orchestration**

1. `repo.find_by_id(&cmd.space_id)` → `EspaceInconnu` si `None`
2. `space.change_member_role(&cmd.actor, &cmd.target, cmd.nouveau_profil)` —
   l'agrégat valide et rend l'événement domaine
3. `repo.update_member_profile(&cmd.space_id, &cmd.target, &cmd.nouveau_profil)`
4. `emettre(bus, event.to_enveloppe())`
5. rendre `MembershipOutcome { administrateurs: space.nombre_d_administrateurs() }`

Le compte est lu **sur l'agrégat muté**, pas relu en base : l'agrégat vient
d'appliquer le changement, il est la source la plus fraîche qui soit.

**Erreurs**

```rust
pub enum ChangeMemberRoleError {
    EspaceInconnu,
    Metier(SpaceMembershipError),   // DernierAdministrateur | ActeurEstLaCible | PasMembre
    Database(String),
}
```

`From<SpaceRepositoryError>` vers `Database`, comme `JoinSpacesError`.
`From<SpaceMembershipError>` vers `Metier` : le use case **ne réinterprète pas**
l'erreur du domaine, il la transporte. C'est le contrôleur qui décidera du
statut HTTP.

## `remove_member_use_case`

```rust
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RemoveMemberCommand,
    repo: &dyn ISpaceRepository,
    bus: &EventBus,
) -> Result<MembershipOutcome, RemoveMemberError>;
```

**Orchestration**

1. `repo.find_by_id(&cmd.space_id)` → `EspaceInconnu` si `None`
2. `space.remove_member(&cmd.actor, &cmd.target)` → événement domaine
3. `repo.delete_member(&cmd.space_id, &cmd.target)`
4. `emettre(bus, event.to_enveloppe())`
5. rendre `MembershipOutcome { administrateurs: space.nombre_d_administrateurs() }`

Le compte sert ici aussi : retirer un administrateur peut faire passer l'espace
à un seul, ce qui fige la ligne du survivant.

**Erreurs** — même forme, `RemoveMemberError { EspaceInconnu, Metier, Database }`.

## Ce que les use cases ne font pas

**Ils n'émettent aucun app event.** `emettre()` publie sur le bus **interne** du
BC ; c'est le publisher de `io/app_events/` qui souscrit, appelle
`to_app_event()` et publie sur l'`app_event_bus`. L'`app_event_bus` n'est jamais
un paramètre de use case — règle du `CLAUDE.md`, et vérifiée par l'axe 12 de
`check-arch`, qui refuse tout `.send(` direct.

Conséquence concrète : le retrait franchit la frontière vers `competitions`
**sans que le use case le sache**. Il émet `UserUnsubscribedFromSpace` sur le bus
interne ; le mapping `to_app_event()` en fait un `UserUnsubscribed` ; le listener
de `competitions` réagit. Trois maillons, aucun couplage.

**Ils ne décident aucune règle.** « Est-ce le dernier administrateur ? »,
« l'acteur est-il la cible ? » — les deux vivent dans l'agrégat. Le use case
charge, appelle, persiste, émet.

**Ils ne connaissent ni HTTP ni HTML.** `MembershipOutcome` porte un `usize`,
pas un fragment.

## Transactions

**Pas de transaction explicite.** `spaces` n'est pas event-sourcé : il n'y a pas
d'append d'événement à rendre atomique avec une projection, seulement une
écriture dans `spaces__user_space`. La règle de transaction unique du
`CLAUDE.md` vise les projections event-sourcées ; elle est sans objet ici.

Le domain event part sur un bus en mémoire **après** l'écriture réussie. Si le
processus tombe entre les deux, l'écriture est faite et l'événement perdu — le
listener de `competitions` ne s'exécute pas, et le coach retiré reste
administrateur de compétition. C'est le comportement de tous les listeners
cross-BC du projet aujourd'hui, pas une régression introduite ici, et la table
`competitions_members` reste reconstructible depuis `spaces__user_space`.
À noter, pas à corriger dans cette carte.

## Instrumentation

`#[tracing::instrument(skip_all, fields(cmd = ?cmd))]` sur les deux — c'est la
règle d'observabilité du `CLAUDE.md`, et l'axe 11 de `check-arch` la vérifie.
`skip_all` est indispensable : sans lui l'attribut tente d'enregistrer `repo`,
qui n'implémente pas `Debug`.

Aucun champ sensible dans les deux commandes, donc pas de `Secret<T>`.

## Tests unitaires

Co-localisés, sur un `FakeRepo` comme `join_spaces`. Un test par règle, plus le
chemin nominal :

| Test | Attendu |
|---|---|
| promotion d'un membre | `Ok`, événement `UserPromotedToSpaceAdmin`, compte +1 |
| rétrogradation d'un administrateur, alors qu'il en reste deux | `Ok`, événement `UserDemotedToSpaceUser`, compte −1 |
| rétrogradation du **dernier** administrateur | `Metier(DernierAdministrateur)`, **aucune écriture, aucun événement** |
| retrait du **dernier** administrateur | `Metier(DernierAdministrateur)`, idem |
| acteur == cible, sur le rôle comme sur le retrait | `Metier(ActeurEstLaCible)` |
| cible non membre | `Metier(PasMembre)` |
| espace inconnu | `EspaceInconnu` |
| retrait nominal | `Ok`, événement `UserUnsubscribedFromSpace` |

**« Aucune écriture, aucun événement » est la moitié qui compte.** Un test qui
vérifie seulement le type d'erreur passerait sur une implémentation qui écrit
d'abord et échoue ensuite. Le `FakeRepo` compte ses appels, et le bus est lu
pour vérifier qu'il est resté vide.

## Question ouverte pour la phase 6

- `nombre_d_administrateurs()` est une lecture sur l'agrégat. Faut-il l'exposer
  publiquement, ou rendre le compte dans l'événement domaine et laisser
  l'agrégat fermé ? La première est plus simple, la seconde évite qu'un appelant
  s'en serve pour reprendre une décision qui appartient au domaine.
