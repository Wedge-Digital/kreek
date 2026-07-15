# Post-match team effects — Spec index

Application des conséquences financières et de popularité d'un rapport de
match publié sur les 2 équipes participantes (`teams` BC). Complète le
câblage minimal posé par la carte 172 (feature `team-state-management`), qui
stubbait ces valeurs à 0.

Pas de nouvelle page front — feature 100% événementielle inter-BC, le
listener `match_report_published_listener` existe déjà (carte 172), cette
feature en corrige/complète le contenu métier.

## Pages

| Page | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| — | ➖ (pas de front) | ✅ | ➖ (réutilise `MatchReportPublishedPayload` existant) | ➖ (pas de use case dédié, listener appelle directement la méthode domaine) | ✅ | ✅ | ✅ |

## Règles métier (validées)

1. `dedicated_fans_équipe = clamp(actuel + fan_mod_du_rapport_pour_cette_équipe, 0, 20)` — la
   valeur du rapport (`FanFactorMod`, déjà bornée -2..2 côté BC `match_report`,
   saisie par le coach à l'étape 5) s'applique **directement**, sans recalcul
   additionnel basé sur le résultat du match (win/nul/défaite).
2. `trésorerie_équipe = actuel + gain_kpo_du_rapport_pour_cette_équipe`.
3. **Jamais de croisement home/away** : l'équipe home reçoit
   `home_fan_mod`/`home_gain_kpo`, l'équipe away reçoit
   `away_fan_mod`/`away_gain_kpo`.
4. `spp_gains` reste hors périmètre (stub `vec![]` inchangé — cartes 35/145/154).
5. `MatchResult` (Win/Draw/Loss) reste stocké dans l'événement
   `PostMatchSequenceStarted` comme fait historique, mais ne sert plus au
   calcul du fan factor — `MatchResult::fan_modifier()` devient mort et est
   supprimé.
6. Agrégat `Team` event-sourcé : la méthode domaine ne fait que calculer et
   retourner l'événement (`dedicated_fans`/`treasury_income` déjà calculés
   dedans) ; c'est `apply()` qui mute l'état à partir de l'événement,
   rejouable à l'identique via `hydrate()`. Le listener ne fait qu'`append()`.
