# Le récapitulatif des coups de pouce terminés

**Épic :** E20 — Coups de pouce : le panier et l'annulation · **Ordre :** 3 · **Dépend de :** 626, 627
**Conception :** `docs/specs/saisie-mobile/E20-cadrage.md` (R3, R6) · **Maquettes :**
`app-mobile-inducements-done.html`, `app-mobile-inducements-cancel.html`

## Objectif

Une fois les deux équipes passées, la page des coups de pouce montre leurs
achats côte à côte, le coup d'envoi, et l'annulation confirmée dans la page —
pas de fenêtre du navigateur. Même route, `/inducements/{team_id}`, autre état.
C'est la dernière vérification avant le match : après l'étape 5, plus rien ne
se corrige.

## Conception

**Quand** : `GET /inducements/{team_id}` quand les deux équipes ont validé,
quel que soit `team_id`, en `PreMatch` comme en `ReadyToPublish`.

**Ce qu'il montre, par camp** (favori d'abord) : nom, rôle (« Favori · achète
en premier », « Outsider · achète en second »), « Terminé ✓ », une ligne par
achat — nom, nature, total —, le total sur le budget
(`inducement_budget_for`, méthode du domaine). « Passer » donne une liste vide :
« Aucun coup de pouce ».

**Les noms** passent par un service d'hydratation dans `use_cases/`, pas par le
contrôleur :
- coups de pouce et vedettes : `IReferenceCatalogPort::find_inducement_labels`
  (le patron de `inducements_print_service.rs:50`) ;
- mercenaires (`MERCO:{poste}:{niveau}`) : `ITeamDataPort::find_roster_positions`
  → `position_name`, et « Mercenaire » / « Mercenaire Niv. 1 » ;
- nature : « Star » pour une vedette engagée, « Mercenaire », sinon « Coup de
  pouce » (décision 6 du 2026-11-01).

**« par … »** (décision 5) : `inducements_recorded_by` par camp
sur `PreMatch` et `ReadyToPublish`, posé au rejeu de `InducementsRecorded`,
effacé par `InducementsCancelled` ; le nom par `ICoachDataPort`.

**Actions** :
- « Coup d'envoi → Match » : lien vers `/step3` ;
- « Annuler les coups de pouce » : **seulement si R3 le permet**
  (`PreMatch`, aucune action) — un prédicat du domaine,
  `can_cancel_inducements()`, pas un recalcul dans la vue ;
- la confirmation remplace les boutons (Alpine, dans la page) : « Garder » /
  « Oui, tout annuler » → `hx-post` vers la route de 626. La phrase « Morgane
  est prévenue » de la maquette n'est pas reprise (E23).

**La seconde validation** mène ici plutôt qu'à l'étape 3 (décision 4) :
`RedirectToStep3` devient `RedirectToRecap`.

**CSS** : dans `pages/match-report-inducements.css`, sous `.match-report-inducements`.

## Données de production

Aucune modifiée (au plus deux champs reconstruits au rejeu). Les rapports où les
deux équipes ont validé montrent le récapitulatif dès la livraison.

## Checklist

- [x] `can_cancel_inducements()` et ses tests
- [x] Le service d'hydratation du récapitulatif et ses tests sur doublures
- [x] Le gabarit, la confirmation dans la page, le lien de coup d'envoi
- [x] La redirection de la seconde validation, si retenue ;
      `test_underdog_pass_redirects_to_step3` suit
- [x] `make lint`, `make check-arch`, `make test`, e2e impactés
