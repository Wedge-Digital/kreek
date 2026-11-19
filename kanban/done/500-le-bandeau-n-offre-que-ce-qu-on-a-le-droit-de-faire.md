# Le bandeau n'offre que ce qu'on a le droit de faire

**Priorité : haute**
**Dépend de :** rien (la carte 389 a posé le mécanisme)
**Fichiers :** `src/app/teams/io/web/team_detail.rs`,
`tests/e2e/test_team_detail_state_banner.py`

## Objectif

Sur la fiche d'équipe, **aucun** bouton d'action du bandeau n'apparaît à un
visiteur qui n'est ni le coach propriétaire, ni un administrateur de l'espace,
ni un administrateur de la compétition où l'équipe est inscrite.

Le bandeau lui-même reste : son état, son texte, son icône, et le bouton
`Imprimer en PDF`. On retire des raccourcis qu'on n'a pas le droit
d'emprunter, on ne cache pas la page.

## L'état actuel — un booléen calculé, appliqué à un CTA sur cinq

La carte 389 a fait tout le travail difficile : `ITeamAccessPort`, son adapter,
et `roster_edit_access_service::peut_modifier_effectif`, qui répond exactement
à la question ci-dessus. `team_detail` le calcule à chaque rendu.

Son périmètre était explicitement `✎ Modifier l'effectif` **et rien d'autre**.
Le booléen n'est donc lu qu'à un seul endroit — `team_detail.rs:119` — et les
quatre autres branches du `match` construisent leur `ctas: vec![…]` en dur :

| Phase | CTA | Gardé aujourd'hui |
|---|---|---|
| `ReadyToPlay` | `✎ Modifier l'effectif` | oui |
| `ReadyToPlay` | `Imprimer en PDF` | sans objet — n'agit sur rien |
| `MatchReporting` | `Reprendre le rapport →` | **non** |
| `PlayerImprovement` | `Évolutions terminées` | **non** |
| `Recruitment` | `Recruter →` | **non** |
| `Dismissals` | `Gérer les renvois →` | **non** |
| `CostlyMistakes` | `Lancer le dé →` | **non** |

Un coach simple qui ouvre la fiche d'un autre y voit « Recruter → » et
« Évolutions terminées ». Et contrairement au cas de la carte 389, ces
boutons-là **marchent** : c'est l'objet de la carte 501.

## La décision : filtrer par construction, pas branche par branche

Ajouter `if peut_editer` dans chacune des cinq branches réglerait le cas du
jour et rien de plus : un sixième état de jeu ajouté demain naîtrait ouvert,
et personne ne s'en apercevrait — c'est précisément par cet oubli-là que la
carte 389 a laissé quatre CTA derrière elle.

Le `match` est donc extrait tel quel dans `pour_etat`, et le filtre est posé
**après**, sur la liste produite :

```rust
fn from_domain(team, space_id, app_routes, peut_editer) -> Option<Self> {
    let mut banner = Self::pour_etat(team, space_id, app_routes)?;
    if !peut_editer {
        // Seul l'impression survit : elle n'agit sur rien. Un état ajouté
        // demain est gardé sans qu'on y pense — c'est tout l'intérêt d'un
        // filtre en sortie plutôt que d'une condition par branche.
        banner.ctas.retain(|cta| matches!(cta, BannerCtaVm::Print));
    }
    Some(banner)
}
```

Le `match` de `ReadyToPlay` cesse alors de lire `peut_editer` : il liste
`[RosterEdit, Print]` sans condition, et le `retain` fait le tri. Une seule
règle, un seul endroit.

**Déplacement par copier-coller** (règle 5) : le corps du `match` n'est pas
réécrit, il est déplacé sous sa nouvelle signature. Effet de bord bienvenu —
`from_domain` repasse sous les 20 lignes, ce qu'elle n'était plus.

Le gabarit ne change pas : il rend les CTA que le VM lui donne, et le script du
bandeau sort déjà proprement quand le déclencheur d'édition est absent
(`if (!declencher) return;`).

## Ce que la carte ne fait pas

- Elle ne garde **aucune** écriture : c'est la carte 501, et les deux vont
  ensemble. Tant que la 501 n'est pas faite, celle-ci ne fait que rendre le
  trou moins visible.
- Elle ne touche pas à `peut_modifier_effectif`, dont la règle est la bonne et
  reste inchangée.
- Elle ne touche pas au bouton `✏️ Customiser` de la fiche joueur, gardé par
  `can_customise`, qui exclut délibérément le propriétaire.

## Checklist

- [x] `pour_etat` extrait par copier-coller, `from_domain` réduit au filtre
- [x] `ReadyToPlay` liste ses deux CTA sans condition
- [x] Un test unitaire **par phase** : bandeau, texte, icône et variante
      identiques, CTA d'action absent pour un tiers
- [x] Contre-épreuve propriétaire dans le même test — sans elle, il passerait
      aussi bien si le bouton avait disparu pour tout le monde
- [x] Un second test pour le bandeau d'attente d'inscription, qui n'a jamais eu
      de CTA : le filtre ne doit pas en faire disparaître le texte
- [x] E2E : le contexte `X-Bypass-Auth-Profile: simple` de
      `test_roster_edition.py:375` croisé avec les cinq phases de
      `test_team_detail_state_banner.py`, **vu échouer**
- [x] `make lint`, `make check-arch`, `make test` — 1653 tests
- [x] `make e2e` — 356 passés, 7 ignorés

## Le test qui compte

Celui qui vérifie que le bandeau **garde tout le reste**. La carte 389 avait
appris cette leçon : un correctif qui viderait les CTA en bloc, ou masquerait
le bandeau entier, passerait le test du bouton absent sans rien corriger. Le
test compare les deux rendus et n'accepte comme différence que la liste des
CTA.


## Ce qui a été fait

**Les deux tests unitaires ont été vus échouer.** En neutralisant le `retain`,
`aucune_phase_n_offre_d_action_a_un_tiers` tombe sur la première phase
vérifiée : c'est bien le filtre qu'il éprouve, et non la forme du VM.

Le test parcourt les six états porteurs de bandeau et compare, pour chacun,
les deux rendus sur quatre champs — titre, détail, icône, variante CSS — en
plus du décompte des CTA. Vérifier le seul décompte aurait laissé passer un
correctif qui masque le bandeau entier.

L'e2e vérifie les cinq phases là où elles sont réellement vivantes, dans le
module qui pilote déjà la séquence par de vraies actions. Les fabriquer dans un
module à part aurait demandé de rejouer toute la séquence une seconde fois.

### L'e2e aussi a été vu échouer

Filtre neutralisé, les quatre tests de phase tombent, et le premier sur
l'assertion qui compte :

```
expect(bandeau.locator(".state-banner-cta:not([onclick])")).to_have_count(0)
Actual value: 1
```

Ce `1`, c'est « Reprendre le rapport → » offert à un membre simple. Filtre
rétabli : quatre passés.

`:not([onclick])` est ce qui distingue les deux moitiés de la règle. « Imprimer
en PDF » est le seul CTA sans URL ni action serveur — il porte son
`onclick="window.print()"` —, et c'est précisément celui qui doit rester. Sans
cette exclusion, le test passerait aussi bien si le bandeau avait été vidé pour
tout le monde.
