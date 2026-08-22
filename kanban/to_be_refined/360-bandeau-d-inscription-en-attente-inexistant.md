# `teams` — un test e2e attend un bandeau qui n'a jamais existé

**Priorité : moyenne** — la suite e2e est rouge en permanence, ce qui use la
confiance qu'on lui accorde
**Trouvée par :** la carte 342, en passant la suite complète
**État : à raffiner** — il faut décider si c'est le test ou la fonctionnalité qui
manque
**Fichiers :** `tests/e2e/test_team_detail_state_banner.py`,
`src/app/teams/io/web/templates/teams-team-detail.html`,
`assets/static/css/pages/team-page.css`

## Le problème

`test_pending_enrollment_banner_is_informational` attend un élément
`.state-banner--pending` sur la fiche d'équipe. **Cette classe n'existe dans
aucun template**, et l'historique git ne montre aucun commit qui l'y aurait
ajoutée puis retirée : elle n'a jamais été rendue.

Le CSS qui la style, lui, existe :

```css
/* pages/team-page.css */
.team-page .state-banner--pending { background: rgba(255,107,53,0.06); }
```

Une règle qui n'a jamais trouvé de markup, et un test qui n'a jamais pu passer.

## Ce que ça coûte

`make e2e` est rouge en permanence. Une suite qui échoue toujours cesse d'être
lue : on prend l'habitude de compter les échecs plutôt que de les traiter, et le
jour où un vrai échec s'y ajoute, personne ne le distingue. C'est le même mode
de défaillance que `CLAUDE.md` décrit à propos des étapes sautées.

## Les questions à trancher

1. **Le bandeau devait-il exister ?** Les autres états de la fiche d'équipe ont
   le leur — le test lui-même en vérifie plusieurs qui passent. Un bandeau
   « inscription en attente » manquant serait une fonctionnalité incomplète, pas
   un test en trop.
2. **Si oui, que doit-il dire ?** Le test l'attend « informationnel », donc sans
   action. Reste à écrire son libellé et à décider dans quel état exact de
   l'agrégat il s'affiche.
3. **Si non**, le test et la règle CSS partent ensemble.

## Ce que la carte ne doit pas faire

**Supprimer le test pour faire passer la suite.** Si le bandeau manque, le
supprimer efface la trace du manque. Le choix doit être explicite dans un sens
ou dans l'autre.

## Checklist — à compléter au raffinage

- [ ] Le sort du bandeau est tranché : à implémenter, ou test et CSS supprimés
- [ ] Si implémenté : l'état de l'agrégat qui le déclenche est nommé
- [ ] `make e2e` passe intégralement
