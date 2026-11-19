# Une erreur JavaScript à l'ouverture du recrutement

> **Carte annulée le 2026-11-11 — doublon de la 592.** Le script fautif est
> celui de la macro `cloudinary_upload`, chargée par le widget du logo de la
> fiche équipe ; la cause prouvée et la correction vivent dans la 592.

**Priorité : moyenne** — un test e2e est rouge ; l'effet pour l'utilisateur
reste à établir
**Trouvée par :** la suite e2e complète lancée pour la carte 622 (2026-11-01).
Sans rapport avec elle : la 622 ne touche que les routes du compte rendu.

## Le symptôme

`test_recruitment_phase.py::test_01_banniere_ouvre_le_catalogue` échoue, seul
comme dans la suite : la fixture `_fail_on_console_errors` de `conftest.py`
relève une erreur de page.

```
TypeError: Cannot read properties of null (reading 'addEventListener')
    at HTMLScriptElement.<anonymous> (<anonymous>:54:53)
```

Le parcours : la fiche équipe en phase de recrutement, puis le clic sur le
bouton du bandeau (`.state-banner--phase .state-banner-cta`), qui échange
`#app-content` par htmx (`hx-get`, `hx-push-url`) pour ouvrir le catalogue.

**Un troisième parcours** (e2e de la carte 631) :
`test_dismissals_phase.py::test_01_banniere_ouvre_l_effectif` — l'effectif ouvert
depuis le bandeau de la fiche équipe.

**Un second parcours** (e2e de la carte 623, 2026-11-01) :
`test_team_print_page.py::test_le_bouton_de_la_fiche_mene_a_la_page` lève la
même erreur en quittant la fiche équipe par le bouton « Imprimer ». Le point
commun : un échange htmx **qui part de la fiche équipe**.

## Ce qu'on sait, et ce qu'on ne sait pas

- `<anonymous>` : c'est un script **inline exécuté par htmx** au moment de
  l'échange, ligne 54 de ce script.
- Ce n'est **pas** le script du bandeau de `teams-team-detail.html`, qui se
  protège (`if (!declencher) return;`).
- Les gabarits du recrutement (`teams-recruitment.html`,
  `widgets/recruitment-catalog.html`) n'appellent pas `addEventListener`.
- Reste à trouver le script : un `pageerror` de Playwright ne donne pas sa
  source. Piste — écouter `htmx:beforeSwap` et lister les `<script>` du
  fragment reçu, ou chercher dans les gabarits que la page compose ceux dont la
  54ᵉ ligne d'un `<script>` porte un `addEventListener`.
- L'impression de la feuille d'équipe (E16, `ecd68304`), fusionnée récemment, a
  ajouté des scripts à la fiche équipe : première piste à vérifier.

## Ce qu'il faut faire

1. Identifier le script, et le parcours réel qui le déclenche.
2. Dire si un utilisateur voit quelque chose de cassé, ou si seule la console
   proteste.
3. Corriger : le script ne s'exécute que là où son élément existe.
