# `app` — Chaque use case dit ce qu'on lui a demandé

**Priorité : haute** — c'est la carte qui répond au symptôme d'origine
**Dépend de :** carte 332 (`Debug` requis), carte 330 (sans `rid`, les lignes
ne se rattachent à rien)
**Fichiers :** les 45 fonctions de `src/app/*/use_cases/` prenant une commande,
`scripts/check-arch.sh`

## Le problème

Le comptage des appels à `tracing` dans `src/` :

| Niveau | Occurrences |
|---|---|
| `error!` | 198 |
| `warn!` | 74 |
| `info!` | 12 |
| `debug!` | 16 |

Sur les 12 `info!`, deux sont le middleware de requête, trois des commandes CLI,
sept des listeners. Autrement dit : **il n'existe aucune trace du chemin
nominal.** Le journal ne sait dire qu'une chose — « ça a cassé ». Quand rien ne
casse mais que le comportement est faux, il se tait ; les deux bugs du mode
customisation (cartes 326 et 327) en sont l'illustration.

Plus précisément : **sur 63 fichiers de `use_cases/`, un seul journalise quoi
que ce soit.** La couche qui sait *ce que l'utilisateur essayait de faire* est
muette, tandis que les 198 `error!` vivent dans les handlers et parlent de
sérialisation et de SQL. Le journal décrit la plomberie, jamais l'intention.

## Ce qu'il faut faire

Un attribut `#[tracing::instrument]` sur chaque fonction publique de use case.

**45 des 54 fonctions publiques `async` de `use_cases/` nomment leur premier
paramètre `cmd` et le typent `…Command`.** L'attribut y est donc rigoureusement
identique — un copier-coller, sans composition à faire fonction par fonction :

```rust
#[instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(cmd: ValidateCustomisationCommand, /* … */) -> Result<…>
```

`skip_all` est indispensable : sans lui, l'attribut tente d'enregistrer **tous**
les paramètres, dépôts compris — or `&dyn IPlayerRepository` n'implémente pas
`Debug`, et ça ne compile pas. `fields(cmd = ?cmd)` remet la seule chose qu'on
veut voir.

Le **nom du use case est gratuit** : `tracing` imprime la cible, c'est-à-dire le
chemin de module (`kreek::app::players::use_cases::validate_customisation_use_case`).
Rien à nommer à la main, rien à maintenir, aucune dérive possible entre le nom
déclaré et le code — et `grep validate_customisation` fonctionne.

### Les neuf autres fonctions

- **Six sont des *services***, pas des use cases : `customisation_basket_hydration_service`,
  `correction_eligibility_service`, `basket_hydration_service` (deux fonctions),
  `team_value_service`. Elles n'ont rien à faire ici — les instrumenter
  produirait du bruit sans intention métier à raconter.
- **Trois sont de vrais use cases à paramètres scalaires** :
  `revert_match_ranking_use_case`, `approve_enrollment`,
  `recompute_team_value_use_case`. Leur attribut nomme explicitement les champs
  à retenir, ou bien on leur donne une commande — à trancher à l'implémentation,
  ce sont trois cas.

### Ce qu'on ne fait pas

**Pas d'option `err`.** Elle journalise en `ERROR` tout retour `Err`, ce qui
classerait un refus métier — `NothingToApply`, `ConcurrentWrite` — comme une
panne. Les `tracing::error!` déjà en place aux sites d'appel couvrent les vrais
échecs techniques. On affinera si le besoin apparaît.

**Pas de reclassement des 198 `error!`.** C'est un chantier distinct, plus long
et moins urgent, à ne surtout pas mélanger à celui-ci.

## Le verrou

Un axe `check-arch` : dans `src/app/*/use_cases/`, une fonction publique `async`
prenant une commande **doit** porter un `#[instrument]` sur la ligne
précédente.

C'est un contrôle facile et fiable, précisément parce que l'attribut décore
**l'appelé** : la vérification porte sur un seul dossier, avec une adjacence de
deux lignes. C'est aussi ce qui rend l'instrumentation impossible à oublier —
tout appel est couvert, qu'il vienne d'un handler, d'un listener, d'un autre use
case ou d'un test. Un dispositif posé sur les 49 sites d'appel n'aurait offert
ni l'une ni l'autre de ces garanties.

## Ce que ça donnera

Aujourd'hui, en production, une validation de customisation réussie ne
journalise **rien**. Après les cartes 330 et 333 :

```
INFO req{rid=01J8QF method=POST path=/spaces/S1/players/P123/customisation/validate coach=Bagouze}
     :execute{cmd=ValidateCustomisationCommand { player_id: PlayerId("P123"), … }}:
     kreek::app::players::use_cases::validate_customisation_use_case: close time.busy=42ms
```

`grep 'rid=01J8QF'` rend la requête entière ; `grep validate_customisation` rend
toutes ses exécutions ; `grep P123` rend tout ce qui a touché ce joueur, tous
BCs confondus.

## Checklist

- [ ] Les 45 fonctions à commande portent `#[instrument(skip_all, fields(cmd = ?cmd))]`
- [ ] Les trois use cases à scalaires sont traités, leur choix documenté dans la
      carte
- [ ] Les six services sont laissés de côté, et la raison est écrite quelque part
- [ ] Axe `check-arch` bloquant, vérifié sur un cas volontairement fautif
- [ ] Vérifié sur une sortie réelle qu'une exécution produit bien sa ligne, avec
      le `rid` de la requête et la durée
- [ ] Vérifié qu'aucune commande de `auth` n'imprime de secret — la carte 332
      doit être faite avant
- [ ] `make test` et `make check-arch` passent
