# Les tests e2e de l'ajustement de trésorerie

**Ordre :** 4 · **Dépend de :** `559`
**Conception :** `docs/specs/ajustement-tresorerie/panneau-d-ajustement/07-integration.md`

## Objectif

Prouver sous Playwright ce qu'aucun test unitaire ne voit : que la garde tient
au-delà du JS, et que le solde affiché à deux endroits bouge ensemble.

## Conception

### On étend `test_team_treasury_tab.py`

Pas de fichier neuf. Sa fixture `tresorerie_ctx` construit une compétition
entière — quatre équipes, un match joué et publié, quatre formes de ligne au
relevé. La dupliquer doublerait le coût de la suite pour rebâtir le même
montage.

Et **l'entrée de la carte d'impact ne bouge pas** : `test_team_treasury_tab` y
liste déjà ses neuf BCs. Un fichier neuf en demanderait une qui listerait
exactement les mêmes.

### Quatre scénarios, et non cinq

> **Le découpage a vieilli entre l'écriture de cette carte et son
> implémentation.** La 559 a livré **sept tests de handler** qui montent le
> routeur de production : membre simple à 403, montant hors bornes, motif vide,
> retrait non couvert, et le crédit de bout en bout — dont une assertion compte
> le solde **trois fois** dans la page rendue. Quatre des cinq scénarios prévus
> ici y sont donc déjà, plus vite et plus précisément.
>
> Les rejouer sous Playwright coûterait des secondes de suite pour prouver à
> nouveau ce qui l'est. Ne restent que les choses qu'un navigateur seul
> établit.

| Scénario | Ce qu'il prouve |
|---|---|
| le panneau s'ouvre au clic | **Alpine vit après le swap d'onglet** — le `x-data` et son `<script>` arrivent dans un fragment échangé |
| les garde-fous du panneau sont branchés | `cfg.max`, `cfg.pas` et `cfg.solde` descendent bien du view model jusqu'à Alpine |
| un crédit appliqué monte les **deux** soldes à l'écran | le swap `#app-content` fait ce qu'on attend — un ciblage trop étroit laisserait l'en-tête sur l'ancien chiffre |
| un membre simple ne voit pas le bouton | en HTTP, avec `X-Bypass-Auth-Profile: simple` |

Le troisième est celui qui vaut la carte : c'est le seul endroit où les deux
affichages du solde sont confrontés **après** une mutation, et aucun test
serveur ne verrait qu'ils se contredisent à l'écran.

### Ce que le second scénario ne peut pas éprouver, et pourquoi

Il devait refuser un retrait que la caisse ne couvre pas. **C'est impossible au
navigateur sur cette fixture**, et la raison tient à une propriété du design
qu'on n'avait pas vue : le plafond est vérifié **avant** le solde, et il vaut
500. Une équipe qui détient 500 kPo ou plus ne peut donc jamais atteindre le
refus « le solde ne couvre pas » — tout montant capable de dépasser sa caisse
dépasse d'abord le plafond.

Ce n'est pas un défaut. Le refus existe, le domaine le porte, et les tests de
handler l'éprouvent sur une caisse vidée par deux retraits. Mais **le garde-fou
client du solde ne sert que les caisses pauvres**, et c'est une chose à savoir
avant de croire le tester.

Le scénario éprouve donc ce qui est atteignable : le plafond, le pas de 5, le
motif obligatoire, et — par l'annonce du nouveau solde, qui le calcule — la
liaison à `cfg.solde`. Une assertion garde l'hypothèse : si la fixture passait
un jour sous 500 kPo, le test le dit au lieu de passer en prouvant autre chose.

### Le profil simple

Le middleware n'échange l'identité **que sur une session vide** : en HTTP par
`requests`, sans cookie, l'en-tête suffit. Dans un navigateur, il faudrait un
contexte neuf — raison de plus pour que les deux scénarios de refus soient des
appels HTTP et non des parcours Playwright.

`test_competition_admin_acces.py` pose déjà le motif :

```python
ENTETE_MEMBRE_SIMPLE = {"X-Bypass-Auth-Profile": "simple"}
```

### Le piège du clic

Le scénario du crédit passe par `cliquer_quand_cable` (`htmx_helpers.py`). Le
panneau est injecté par un swap htmx, donc il traverse la fenêtre où un élément
est **peint, cliquable et inerte** : un clic nu s'y perdrait sans requête, sans
erreur de console, sans rien. Pas de `sleep` — la condition rend la main en 7 à
20 ms.

### La base n'est pas relue ici

La carte prévoyait de confronter l'écran à `teams__treasury_ledger`. C'est fait
**ailleurs et mieux** : la 558 a un test d'intégration sur vraie base qui
vérifie le motif, le sens, le montant, le solde après et la charge utile jointe.

Ce que l'écran apporte en plus, c'est la confrontation des **deux affichages du
solde** — l'agrégat dans l'en-tête, le grand livre dans le bandeau — après une
mutation. Y ajouter une troisième source affaiblirait le test, comme le dit déjà
`test_le_solde_du_releve_egale_celui_de_l_en_tete` juste au-dessus.

## Checklist

- [ ] Les quatre scénarios dans `test_team_treasury_tab.py`
- [ ] `cliquer_quand_cable` pour atteindre l'onglet, aucun `sleep`
- [ ] ~~Vérification en base du `reason` et de `balance_after_kpo`~~ — faite par
      le test d'intégration de la 558, sur une vraie base
- [ ] ~~`HX-Request: true` sur les POST directs~~ — plus de POST direct ici
- [ ] `make e2e` au vert, et la carte d'impact vérifiée inchangée
