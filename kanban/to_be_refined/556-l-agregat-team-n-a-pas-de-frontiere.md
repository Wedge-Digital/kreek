# L'agrégat `Team` n'a pas de frontière

**Priorité : moyenne**
**Dépend de :** rien
**Contexte :** `teams` — dette de conception
**Relevé par :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/06-domaine.md`
**Entamé par :** carte `557` — `treasury` est le premier champ passé en privé

## Le constat

`Team` est l'agrégat le plus gardé du projet — vingt méthodes de commande, qui
vérifient la phase, le statut, le budget, les quotas — et **vingt-quatre de ses
vingt-cinq champs sont `pub`**.

Le vingt-cinquième, `treasury`, a été fermé par la carte 557 : il se lit par
`treasury()` et ne s'écrit que par `apply()`. Il reste donc vingt-quatre.

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
| champs publics restants | **24** |
| lectures de champ hors de l'agrégat | **~125** |
| fichiers concernés | **33** |

Ce n'est pas un correctif, c'est un chantier — mais il **se fait par morceaux**,
et la 557 l'a prouvé.

Cette carte a d'abord soutenu l'inverse : « il ne se fait pas à moitié, un champ
privé sur vingt-cinq donnerait l'illusion d'un agrégat gardé ». L'argument n'a
pas tenu à la mesure. Fermer `treasury` seul a coûté **14 substitutions
mécaniques**, n'a cassé aucun test, et rien dans le code ne prétend que les
autres champs sont gardés. L'illusion était rhétorique ; le bénéfice, lui, est
réel — `adjust_treasury` et `recruit_player` ne se contournent plus.

Et le patron existe déjà dans la maison : `RecruitmentBasket`, dans le même
dossier `domain/`, a **tous** ses champs privés. C'est `Team` qui est
l'exception.

## Ce qui reste à définir

- **Tout privé, ou seulement les champs porteurs d'invariants ?** `game_phase`,
  `participation_status` et les compteurs de staff en portent. `name`,
  `logo_url`, `coach_name` sont des libellés dénormalisés que personne ne peut
  corrompre dangereusement. **La question reste entière** — `treasury` ne la
  tranche pas, il était du premier groupe.
- **Quelle forme d'accès en lecture ?** `treasury()` **y répond par l'exemple** :
  l'accesseur unitaire se lit bien, et les 14 sites n'ont demandé aucune
  réécriture. Reste à savoir si ça tient encore sur 125, ou si les VMs de
  `teams` — qui lisent `team.<champ>` en quantité — gagneraient à recevoir un
  view model de lecture que l'agrégat produirait d'un coup.
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

Un détail à régler au passage : le commentaire de `equipe_en_renvois`, dans les
tests de `team.rs`, annonce que « la trésorerie est posée par un événement de
crédit plutôt qu'écrite dans l'agrégat » — et la ligne suivante l'écrit
directement. Le texte dit la bonne intention, le code fait l'inverse.

## Checklist (à compléter après raffinage)

- [ ] Trancher : tout privé ou les seuls champs porteurs d'invariants
- [ ] Trancher la forme d'accès en lecture — accesseurs, ou VM de lecture
- [ ] Aligner le commentaire et le code d'`equipe_en_renvois`
- [ ] Auditer les autres BCs avant de décider d'un axe `check-arch`
- [ ] Découper en cartes — 125 sites ne tiennent pas dans une session
