# Panneau d'ajustement · Phase 7 : effets de bord

**Entrée** : `06-domaine.md` validé.

## Persistance — aucune méthode neuve

`find_by_id` et `append` suffisent.

`append` écrit **l'event store, la projection et le grand livre dans une seule
transaction** (`team_repository.rs:390-408`), puis émet après le commit.
L'insertion de la ligne de grand livre est pilotée par `treasury_movement()` :
elle vient toute seule, sans que le chemin d'écriture connaisse l'ajustement.

**Aucun bras de projection à écrire.** `team_proj` ne porte pas de colonne
trésorerie — la fiche la lit de l'agrégat hydraté — et le `match` de
`update_projection_in_tx` se termine par `_ => {}`. Il n'y a donc même pas de
bras vide à poser.

`list_treasury_movements` rend déjà le `payload` de l'événement, joint depuis
`team_event_store`. C'est de là que `detail_de()` tirera « Par Bagouze — … ».

## Événements — aucun listener à câbler

L'événement ne sort pas du BC (phase 3), et **aucun listener intra-BC ne doit y
réagir**.

En particulier, il ne rejoint **pas** `ends_in_ready_to_play()`. La valeur
d'équipe est une somme de joueurs et de staff où la trésorerie n'entre pas : il
n'y a rien à recalculer.

C'est l'inverse du cas documenté par la carte 46, où un `GamePhaseOverridden`
laisserait une TV périmée s'il n'était pas ajouté à cette liste. Écrit ici pour
qu'on ne fasse pas le rapprochement à tort — les deux événements sont des outils
d'admin, et c'est leur seul point commun.

## Le handler

```rust
pub async fn adjust_treasury(
    Path((space_id, team_id)): Path<(String, String)>,
    perms: SpacePermissions,
    State(state): State<AppState>,
    auth_session: AuthSession,
    Form(form): Form<AdjustTreasuryForm>,   // en dernier — axum l'exige
) -> Response
```

`Form` consomme le corps de la requête : axum impose qu'il soit le dernier
extracteur. Posé ailleurs, le code ne compile pas — autant le savoir avant.

Découpé en `construire_commande` et `repondre_erreur` pour tenir sous vingt
lignes.

**Le middleware CSRF maison exige `HX-Request: true`** sur tout POST. htmx le
pose ; un test qui poste en direct doit le poser aussi, comme le font déjà les
tests e2e du dépôt.

### Les codes de retour

| Cas | Réponse |
|---|---|
| le visiteur n'est pas admin | **403** |
| sens illisible | **400** — le formulaire n'envoie que `Credit` ou `Debit` ; une autre valeur vient d'ailleurs que de l'écran |
| montant hors bornes, ou motif vide / trop long | **200** + pied d'erreur retargeté |
| `InsufficientTreasury` | **200** + pied d'erreur |
| `ConcurrentWrite` | **200** + pied d'erreur — « la fiche a changé, recommence » |
| équipe introuvable | **404** |
| autre erreur de dépôt | **500**, journalisée |

**200 et non 4xx pour les refus de saisie** : htmx n'échange pas une réponse
non-2xx par défaut. Un 422 n'afficherait rien, et le bouton paraîtrait sans
effet. Les deux points d'erreur existants du dépôt répondent en 200 pour cette
raison exacte.

Le **400** du sens illisible est l'exception assumée : ce n'est pas une erreur
d'usager, donc il n'y a pas de message à lui montrer dans le panneau.

## Templates

| Gabarit | Rôle |
|---|---|
| `teams-treasury-tab.html` | le bouton et le panneau, sous `{% if let Some(adjust) = vm.adjust %}` |
| le pied du panneau | rendu **dans la page** et **seul** en cas d'erreur |

**Un point à régler à l'implémentation.** Askama résout `{% include %}` contre le
contexte du gabarit incluant : les noms de champs doivent concorder entre le
panneau et le fragment d'erreur rendu seul. Si ça ne tombe pas juste, on assume
**six lignes dupliquées** plutôt que de tordre les view models pour satisfaire un
mécanisme de gabarit. Noté comme un choix à faire, pas promis comme résolu.

## Tests E2E

### On étend `test_team_treasury_tab.py`, on ne crée pas de fichier

Sa fixture `tresorerie_ctx` construit une compétition entière — quatre équipes,
un match joué et publié, quatre formes de ligne au relevé. La dupliquer
doublerait le coût de la suite pour reconstruire le même montage.

Et l'entrée de la carte d'impact existe déjà, avec ses neuf BCs : elle ne bouge
pas. Un fichier neuf en demanderait une, qui listerait exactement les mêmes.

### Les scénarios

| Scénario | Ce qu'il prouve |
|---|---|
| un membre simple ne voit pas le bouton | en HTTP avec `X-Bypass-Auth-Profile: simple`, le markup du panneau est absent du relevé |
| son POST est refusé | **403** — la garde n'est pas qu'à l'affichage |
| un crédit s'applique | le solde monte **dans le bandeau et dans l'en-tête**, et la ligne apparaît en bas avec son motif et son auteur |
| un retrait non couvert est refusé | message dans le pied, panneau toujours ouvert, solde inchangé |
| un motif vide posté en direct est refusé | la garde n'est pas qu'en JS |

**Les deux derniers sont les seuls à prouver ce qu'aucun test unitaire ne voit
déjà** : que les garde-fous Alpine ne sont pas la seule défense. Les trois
premiers éprouvent le câblage.

### Le piège du clic

Le scénario du crédit passe par `cliquer_quand_cable` (`htmx_helpers.py`). Le
panneau est injecté par un swap htmx, donc la fenêtre « visible mais pas encore
câblé » s'applique : un clic nu s'y perdrait **sans requête, sans erreur de
console, sans rien**. C'est le piège que le `CLAUDE.md` documente, et il a déjà
coûté deux diagnostics faux.

### Deux vérifications dans la base

Le scénario du crédit lit aussi le grand livre directement — `query_db` sur
`teams__treasury_ledger` — pour vérifier que la ligne porte le bon `reason` et
que `balance_after_kpo` concorde avec ce que l'écran affiche. L'écran peut
mentir sur une valeur juste ; c'est exactement le défaut que les cartes 492 à
495 ont produit.

## Règles métier

**Aucune nouvelle.** Question posée le 2026-10-01 : la phase 7 décrit des effets,
pas des permissions.
