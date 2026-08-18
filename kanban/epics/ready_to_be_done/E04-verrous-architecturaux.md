# E04 — Les verrous architecturaux

**État :** 5 cartes · 0 faite

## La fonction

Le projet tient deux invariants par des `grep` bloquants dans
`scripts/check-arch.sh` : **un BC ne lit pas les tables d'un autre**, et **une
ressource n'est atteignable que depuis son espace**. Les deux verrous existent,
et les deux mentent aujourd'hui — le premier par une liste de tolérance, le
second par une couverture incomplète.

Un verrou qui tolère est pire qu'un verrou absent : il dit vert sur du rouge
connu, et le prochain lecteur croit l'invariant tenu.

L'épic vide la liste de tolérance de l'axe 3 et étend le cloisonnement des
espaces à tous les BCs.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 300 | `team_creation` lit directement les repositories de `competitions` | 5 accès directs → un port ; sort de `AXE3_BASELINE_REGEX` |
| 301 | `team_selection_tester.rs` contourne la souveraineté vers `spaces` | 1 accès ; sort de la même liste |
| 07 | Port structs stringly-typés dans le domaine | les ports parlent le langage du domaine, pas celui de la DB |
| 316 | Audit — l'appartenance à l'espace est-elle vérifiée dans les autres BCs ? | l'inventaire, prérequis de 323 |
| 323 | Axe `check-arch` — le cloisonnement des espaces ne doit pas se reperdre | le verrou qui empêche la rechute |

## Ce qui commande l'ordre

**316 avant 323** : on ne pose pas un axe bloquant avant de savoir ce qu'il va
faire rougir. L'audit produit l'inventaire, l'axe le verrouille.

**300 et 301 sont indépendantes l'une de l'autre**, mais aucune n'est finie
tant que son entrée n'est pas retirée de `AXE3_BASELINE_REGEX` et que
`make check-arch` ne passe pas sans elle. C'est la checklist des deux cartes,
et c'est ce qui fait la valeur de l'épic — sinon on déplace du code sans
resserrer le verrou.

**301 commence par une question, pas par du code** : la page de test
développeur est-elle encore utilisée ? Si non, la supprimer fait disparaître la
violation sans créer ni port ni adapter. C'est la seule issue qui ne laisse
aucune dette, et elle se regarde en premier.

**07 est indépendante** des quatre autres.

## Ce que l'épic ne couvre pas

- **L'autorisation.** `space_scope_middleware` répond à « de quoi parle-t-on ? »,
  pas à « qui a le droit ? » — il le dit lui-même en commentaire. Les contrôles
  de rôle (admin d'espace, admin de compétition) sont un sujet distinct, qui
  n'a pas encore sa carte.
- **Le découpage en crates cargo.** Écarté par la carte 242 : les verrous
  restent des `grep` qui ne voient ni les chaînes littérales ni le SQL. C'est le
  prix de cette décision, assumé tel quel.
- **Le statut « BC extractible »** (`auth`, `spaces`) et son axe 9, qui tient
  déjà sans tolérance.

## Terminé quand

`AXE3_BASELINE_REGEX` est vide, `make check-arch` passe sans aucune ligne de
base, et un axe vérifie que toute route portant un `{space_id}` a un résolveur
d'appartenance.
