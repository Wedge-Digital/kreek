# L'agrégat `Team` n'a pas de frontière

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `teams` — dette de conception
**Relevé par :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/06-domaine.md`

## Le constat

`Team` est l'agrégat le plus gardé du projet — vingt méthodes de commande, qui
vérifient la phase, le statut, le budget, les quotas — et **ses vingt-cinq
champs sont `pub`**.

```rust
pub struct Team {
    …
    pub treasury: Kpo,
    pub game_phase: Option<GamePhase>,
    pub participation_status: ParticipationStatus,
    …
}
```

Chaque invariant qu'une méthode protège est donc contournable par une
affectation directe. `recruit_player` refuse un joueur que la caisse ne couvre
pas ; `team.treasury = Kpo(9999)` ne refuse rien.

Le `CLAUDE.md` l'écrit à l'envers, sous « Conventions domaine » :

> Agrégats : n'exposent pas de référence mutable vers leur état interne

C'est exactement ce qui n'est pas tenu, et aucun axe de `check-arch` ne le voit.

## Ce que ça coûte de le corriger

Mesuré le 2026-10-01 :

| | |
|---|---|
| champs publics | **25** |
| lectures de champ hors de l'agrégat | **139** |
| fichiers concernés | **33** |

Ce n'est donc pas un correctif, c'est un chantier. Et il ne se fait **pas à
moitié** : un champ privé sur vingt-cinq donnerait l'illusion d'un agrégat gardé
sans en être un, ce qui est pire que l'état actuel — qui, lui, ne prétend rien.

## Ce qui reste à définir

- **Tout privé, ou seulement les champs porteurs d'invariants ?** `treasury`,
  `game_phase`, `participation_status` et les compteurs de staff en portent.
  `name`, `logo_url`, `coach_name` sont des libellés dénormalisés que personne
  ne peut corrompre dangereusement.
- **Quelle forme d'accès en lecture ?** Des accesseurs `fn treasury(&self) ->
  Kpo` sur les 139 sites, ou un view model de lecture que l'agrégat sait
  produire — les VMs de `teams` lisent déjà `team.<champ>` en quantité.
- **Que devient `Default for Team` ?** Il construit l'agrégat champ par champ,
  et `hydrate` part de lui. Un constructeur privé change ce chemin.
- **Faut-il un axe `check-arch` ?** Un `grep` sur `pub ` dans les structs de
  `domain/model/` attraperait la récidive, mais `teams` n'est pas le seul BC
  concerné — l'audit reste à faire.

## Pourquoi elle n'est pas urgente

Aucun défaut connu n'a été causé par cette ouverture : les écritures directes
passent toutes par `apply()`, qui rejoue des événements que le domaine a
produits. C'est une **porte ouverte que personne n'a franchie**, et le risque
est qu'un contributeur pressé la franchisse sans voir qu'il contourne vingt
gardes.

## Checklist (à compléter après raffinage)

- [ ] Trancher : tout privé ou les seuls champs porteurs d'invariants
- [ ] Trancher la forme d'accès en lecture
- [ ] Auditer les autres BCs avant de décider d'un axe `check-arch`
- [ ] Découper en cartes — 139 sites ne tiennent pas dans une session
