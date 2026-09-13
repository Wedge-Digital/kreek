# Widget d'édition du logo d'équipe

**Priorité : moyenne**
**Dépend de :** carte 508, carte 509
**Fichiers :** `src/app/teams/io/web/widgets/team_logo_widget.rs` (nouveau),
`src/app/teams/io/web/templates/widgets/team-logo-widget.html` (nouveau),
`src/app/teams/routes.rs`, `src/app/teams/router.rs`

## Constat vérifié avant d'écrire cette carte

Aucun widget mono-champ n'existe déjà à copier tel quel. Le plus proche est
`src/app/competitions/io/web/admin/settings/general_panel.rs` +
`templates/admin/widgets/settings-general.html` : squelette architectural à
suivre (GET rend, POST se remplace lui-même), mais il porte 3 champs — pas un
composant à copier-coller, une référence de pattern.

## Plan

Widget isolé, sur le squelette de `settings-general` :

- `GET` : rend le widget (logo actuel ou fallback initiales + bouton
  d'édition), **uniquement si le viewer a le droit** (`peut_modifier_effectif`
  — cf. carte 511 pour la décision d'affichage). Si le widget est monté dans
  une route déjà couverte par `garde_action_equipe` (`routes_d_action()`),
  aucune vérification supplémentaire à écrire ici — la garde de groupe s'en
  charge, ne pas la dupliquer (CLAUDE.md : « on garde par construction, pas
  par vigilance »).
- `POST` : valide via `CloudinaryImage::try_new`, appelle
  `change_team_logo_use_case::execute`, puis **relit systématiquement depuis
  la base** avant de rendre le widget (pattern `settings-general` : jamais
  renvoyer la saisie brute non persistée).
- Template : racine avec `hx-disinherit="*"`, formulaire
  `hx-post="{{ routes.teams.team_logo(space_id, team_id) }}"
  hx-target="#team-logo-widget" hx-swap="outerHTML"`, réutilise la macro
  `{% call cmp::cloudinary_upload("logo_url", vm.logo_url, "teams/logos", "Logo", vm.logo_error) %}`.
- **Retrait du logo** : un bouton « Revenir aux initiales », visible seulement
  si un logo est déjà défini, qui soumet le même formulaire avec un champ
  vide/absent — le handler POST interprète l'absence de valeur Cloudinary
  comme `None` (retrait), pas comme une erreur de validation.
- Route : nouvelle entrée dans `routes.rs` (`path::TEAM_LOGO`), montée dans
  `routes_d_action()` du router `teams` — donc automatiquement protégée par
  `garde_action_equipe` (déjà = propriétaire OU admin espace, rien à changer
  côté autorisation).

## Ce que la carte ne fait pas

- Elle ne redéfinit pas la règle d'autorisation (`peut_modifier_effectif`
  reprise telle quelle, via la garde de groupe existante).
- Elle ne crée pas de nouveau mécanisme d'upload — la macro Cloudinary
  existante est réutilisée sans modification.

## Checklist

- [ ] Route `TEAM_LOGO` déclarée + testée (placeholders `space_id`/`team_id`, cf. tests existants de `routes.rs`)
- [ ] Montée dans `routes_d_action()` — protégée par `garde_action_equipe` sans garde locale dupliquée
- [ ] `get_team_logo_widget` : rend le widget actuel
- [ ] `post_team_logo_widget` : valide, exécute le use case, relit depuis la base, rend le widget à jour
- [ ] Bouton de retrait visible seulement si un logo est déjà défini, soumet bien `None` au use case
- [ ] Erreur de validation affichée sous le champ, pas globale
- [ ] `make lint`, `make check-arch`, `make test`
