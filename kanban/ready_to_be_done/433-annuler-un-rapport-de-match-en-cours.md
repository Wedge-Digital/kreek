# Annuler un rapport de match en cours

> Le domaine sait annuler depuis toujours. Aucune route n'y mène.

**Priorité : moyenne-haute** — un rapport ouvert par erreur verrouille les deux
équipes jusqu'à ce qu'un administrateur vide la journée
**Dépend de :** 427 — le listener d'annulation de `competitions` et la
distinction des appariements

## Le symptôme

Un rapport ouvert sur les mauvaises équipes, ou sur la mauvaise journée, **ne
peut pas être abandonné**. Il verrouille la saisie des deux équipes, apparaît aux
résultats, et la seule sortie est qu'un administrateur supprime l'appariement —
ce qui, pour un match programmé, efface la rencontre du calendrier.

Un rapport **manuel** n'a même pas cette sortie : personne ne peut supprimer un
appariement qu'aucun écran ne montre.

## Ce qui existe déjà

```
match_report_draft.rs:74             pub fn cancel(reason) -> MatchReportDomainEvent
match_report_pre_match.rs:52         pub fn cancel(reason)
match_report_ready_to_publish.rs:63  pub fn cancel(reason)
```

L'app event `MatchReportCancelled` est publié, et `teams` l'écoute pour libérer
son verrou de saisie. **Le seul appelant de `cancel()` est
`pairing_deleted_listener`** : la capacité est complète, elle n'a pas de porte.

## Le bouton

Il apparaît sur **`PreMatch` et `ReadyToPublish`**, dans le bandeau du rapport,
avec confirmation.

**Pas seulement en phase 2** : le domaine sait annuler depuis ces deux états, et
restreindre à une étape rendrait incurable un rapport bloqué en phase 3 ou 4 —
précisément là où l'on s'aperçoit qu'on s'est trompé d'équipes.

**Pas sur `Draft`** : un brouillon ne verrouille encore rien, et son écran de
sélection a déjà un retour. Le domaine l'autorise, l'interface ne l'offre pas.

Libellé : **« Annuler le rapport »**, jamais « Supprimer ». Rien n'est effacé —
l'event store garde l'histoire et l'agrégat passe en `Cancelled`. Le mot doit
dire ce que le système fait.

## L'autorisation est déjà écrite

`recap_controller.rs:235` porte exactement la règle voulue — admin d'espace,
admin de compétition, ou coach de l'une des deux équipes :

```rust
if deps.space_admin.is_space_admin(…) { return true; }
if is_competition_admin(…)            { return true; }
is_coach_of_either_team(…)
```

**Ne pas en écrire une seconde.** Trois prédicats identiques divergeraient, et
c'est le genre d'écart qui donne un bouton visible sur une action refusée — la
carte 389 vient d'en corriger un.

Il faut donc **sortir ce prédicat de `recap_controller`** vers un service de
`use_cases/`, et l'appeler des deux endroits.

## La raison

`cancel(reason: String)` en exige une. **Le coach n'en saisit pas** : le geste
est explicite, et la question ralentirait l'abandon d'un rapport ouvert par
erreur. La raison est posée par le contrôleur — qui a annulé, pas pourquoi — et
sert au journal.

## Ce que `competitions` en fait

Traité par la carte 427, mais c'est **cette carte qui le rend atteignable** :
jusqu'ici l'annulation ne survenait qu'après suppression de l'appariement, donc
la distinction ci-dessous ne pouvait pas se produire.

| | |
|---|---|
| Rapport **programmé** | l'appariement reste au calendrier → sa ligne repasse en `upcoming` |
| Rapport **manuel** | l'appariement n'existait que pour lui → supprimé, ligne comprise |

## Ce que la carte ne couvre pas

L'annulation d'un rapport **publié**. Le domaine ne l'expose pas, et défaire une
publication a déjà son chemin — la dépublication.

## Tests

**Unitaires**
- `un_rapport_en_pre_match_s_annule`
- `un_rapport_pret_a_publier_s_annule`
- `un_rapport_publie_refuse_l_annulation`, avec une ligne de journal
- le prédicat partagé rend vrai pour l'admin d'espace, l'admin de compétition et
  les deux coachs, faux pour un coach tiers

**E2E**
- Un coach annule son rapport en cours : l'équipe redevient saisissable et le
  match quitte l'onglet Résultats.

C'est le seul test qui traverse `match_report`, `teams` et `competitions` — et
la libération du verrou passe par un bus d'événements qu'aucun test unitaire ne
franchit.

## Checklist

- [ ] Prédicat d'autorisation sorti de `recap_controller` vers `use_cases/`,
      deux appelants
- [ ] Route et contrôleur d'annulation, un seul use case
- [ ] Bouton dans le bandeau, avec confirmation, sur `PreMatch` et
      `ReadyToPublish`
- [ ] Un `Published` est refusé, avec une ligne de journal
- [ ] Les tests unitaires et le test e2e
- [ ] `make lint`, `make check-arch`, `make test`, `make e2e`
