# Phase 6 — Domaine : ce que cette unité a changé, et rien de plus

**L'agrégat vit dans `onglet-presences/06-domaine.md`**, conçu d'un bloc pour les
trois unités. Cette page ne le redéfinit pas — elle inscrit ce que la troisième
unité lui a fait, parce que c'est le seul endroit où on ira le chercher.

## Ce que cette unité a ajouté au domaine

**Une variante, une erreur, un contrôle** — R28, apparue en phase 2 :

```rust
pub enum Repondant { Jeton, Coach(CoachId), Organisateur(CoachId) }
```

| Ajout | Où |
|---|---|
| la variante `Jeton`, séparée de `Coach(CoachId)` | `Repondant` |
| `TeamNotOwnedByCoach { team }` | `DomainError` |
| le refus d'un `Coach(id)` étranger à la réponse | `enregistrer` |

Le détail, ses motifs et sa conséquence sur la persistance — **le canal ne se
persiste pas** — sont dans `onglet-presences/06-domaine.md`, section R28.

## Ce qu'elle n'a pas eu à ajouter

| Ce que l'encart appelle | Existait depuis |
|---|---|
| `enregistrer(team, venue, Repondant::Coach(id), journee, maintenant)` | carte 512 |
| `statut(maintenant)` — le filtre des campagnes ouvertes | carte 511 |
| `compte_presents()` — le « 9 équipes ont déjà confirmé » | carte 511 |
| `Presence`, `Venue`, `EffetReponse` | carte 511 |
| `EffetReponse::EnregistreeRencontreARefaire` — R30 | carte 512 |

Aucune méthode nouvelle, aucun value object nouveau, aucun test de domaine
nouveau hors celui de R28.

## Le bilan du pari « l'agrégat se conçoit d'un bloc »

Il vaut d'être écrit, parce que c'est le genre de pari qu'on refait.

| Unité | Ce qu'elle a demandé au domaine |
|---|---|
| `onglet-presences` | tout — c'est elle qui l'a conçu |
| `reponse-coach` | **rien**, à la variante `Jeton` près, venue plus tard |
| `encart-competition` | **une variante, une erreur, un contrôle** |

Deux unités sur trois n'ont rien coûté. La troisième a coûté un enum élargi sur
un agrégat de neuf méthodes — à comparer avec ce qu'aurait produit une
conception unité par unité : `enregistrer` conçue pour le seul organisateur
n'aurait pas porté `Repondant` du tout, et les deux unités suivantes auraient
chacune rouvert sa signature.

### Ce qui a été manqué, et la règle qu'on en tire

La forme était bonne — un seul point d'écriture, un champ qui dit **qui**
répond. Ce qui manquait, c'est que deux appelants pouvaient partager une
intention (« le coach répond ») sans partager leur **autorisation** : le jeton
vaut par lui-même, la session doit être confrontée au propriétaire de l'équipe.

La phase 6 de l'unité 1 avait énuméré les appelants par ce qu'ils *veulent*.
Elle aurait dû les énumérer aussi par **ce qui les autorise** :

> Pour chaque appelant d'une méthode de commande : qu'est-ce qui lui donne le
> droit d'appeler, et l'agrégat peut-il le vérifier ?

Sur cet agrégat, la réponse était trois autorisations distinctes et un domaine
qui pouvait vérifier deux d'entre elles — `Reponse` portait déjà `coach_id`. La
question n'avait simplement pas été posée.
