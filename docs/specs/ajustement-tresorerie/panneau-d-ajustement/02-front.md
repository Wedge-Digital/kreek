# Panneau d'ajustement · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-team-treasury-admin.html`

## Ce n'est pas un widget, et le `CLAUDE.md` le dit

Sa section « Quand NE PAS appliquer » le patron d'assemblage vise exactement ce
cas : « page simple avec un formulaire et une réponse ». Il y a deux zones — le
panneau et le relevé — pas trois sections interactives indépendantes, et un seul
BC.

Le bouton et le panneau sont donc rendus **par le fragment d'onglet lui-même**,
`teams-treasury-tab.html`, qui est déjà le gabarit du relevé.

Conséquence côté back, à détailler en phase 3 : `treasury_tab::rendre_onglet`
prend aujourd'hui `(team_id, state)` et devra recevoir en plus `space_id` — pour
fabriquer l'URL du POST — et le droit du visiteur.

### Ce que ce choix évite

Un widget aurait son endpoint GET, donc une seconde requête au chargement de
l'onglet, et un instant où le relevé s'affiche sans le bouton qui le surmonte.
Il aurait surtout à se faire prévenir de l'écriture pour que le relevé se
rafraîchisse — un événement DOM, un abonnement, deux allers-retours — là où une
seule réponse rend les deux à jour.

## Le fragment, et son seul endpoint de mutation

| Élément | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|
| contenu de l'onglet Trésorerie | `team_treasury` (GET, existant) | clic sur l'onglet, ou URL chargée directement | — | lecture, **plus le panneau si admin** |
| panneau d'ajustement | `team_treasury_adjust` (POST, neuf) | soumission du formulaire | — | mutation |

**Aucun événement DOM, ni émis ni écouté.** La règle 2 des widgets régit des
widgets qui se parlent ; il n'y a ici qu'un seul acteur, et la réponse du POST
re-rend tout ce qui a changé.

**Aucun `hx-disinherit`.** La règle protège un widget d'une page hôte porteuse
d'attributs HTMX. Vérifié : ni `teams-team-detail.html` ni `app-layout.html` ne
posent de `hx-vals`, `hx-headers`, `hx-include` ou `hx-params`. Poser
l'attribut par réflexe protégerait d'un danger qui n'existe pas.

## La réponse en cas de succès

```
POST /app/{space_id}/teams/{team_id}/treasury/adjust
```

Elle rend **la page entière**, `active_tab = "treasury"` — c'est
`rendre_fiche(…, "treasury")` réutilisé tel quel, sans gabarit neuf.

```html
hx-post="…"
hx-target="#app-content"  hx-select="#app-content"  hx-swap="outerHTML"
```

**`#app-content` et non `#team-tab-zone`**, pour une raison qu'on ne voit qu'en
lisant le gabarit : la trésorerie s'affiche **deux fois** sur cette page. Dans
le bandeau de l'onglet, et dans l'en-tête de la fiche
(`teams-team-detail.html:59`), qui est **hors** de la zone d'onglets. Un swap
limité à la zone laisserait l'en-tête annoncer l'ancien solde à côté du nouveau
— deux chiffres contradictoires à l'écran, et c'est le plus visible des deux qui
serait faux.

`#app-content` est déjà l'idiome du lien de retour, dix lignes plus haut dans le
même gabarit, et celui du menu principal.

Le POST **ne pousse pas d'URL** : elle reste celle de l'onglet Trésorerie, qui
est exactement ce que la réponse affiche. Le retour arrière du navigateur n'a
donc rien à défaire.

## La réponse en cas d'erreur

Le panneau reste ouvert, la saisie reste à l'écran, et le message s'affiche dans
son pied. Trois en-têtes, sur le modèle de `submit_error_response`
(`finalize_team.rs:51-62`) :

```
HX-Retarget: #adm-foot     HX-Reselect: #adm-foot     HX-Reswap: outerHTML
```

**`HX-Reselect` n'est pas une précaution, c'est le piège déjà payé une fois.**
`finalize_team.rs:44-50` le documente : le formulaire porte `hx-select`, qui
filtre aussi la réponse d'erreur, n'y trouve pas `#app-content`, et **rien ne
s'affiche** — le bouton paraît sans effet. `HX-Reselect` remplace le sélecteur
pour cette réponse-là.

**Statut 200, pas 4xx.** Htmx n'échange pas une réponse non-2xx par défaut ; les
deux points d'erreur existants du dépôt répondent en 200 pour cette raison.

Le corps est le pied du panneau re-rendu, portant le message — donc un gabarit
de fragment à part, assez petit pour être inclus par le gabarit du panneau et
rendu seul par le handler d'erreur.

## Ce qui reste au front

Un `x-data` Alpine sur la racine du panneau, et rien d'autre :

- l'ouverture et la fermeture du panneau ;
- le sens, `Créditer` ou `Débiter` ;
- l'annonce vivante — « le solde passera de 85 kPo à 205 kPo » ;
- les refus de saisie, dans l'ordre où ils bloquent : montant positif, multiple
  de 5, plafonné à 500, retrait couvert par le solde, motif saisi.

**Alpine et non un `<script>` nu.** Le panneau est remplacé à chaque swap de
l'onglet ; un `x-data` se ré-initialise de lui-même, là où un script posé à côté
de son markup survivrait au remplacement du DOM. C'est la règle 7 des widgets
appliquée à un composant qui n'est pas un widget, pour la même raison.

**L'état ouvert/fermé n'a rien à conserver.** Après une écriture réussie, la
page est re-rendue et le panneau revient fermé — sans qu'on ait à le lui dire.

**Ces garde-fous sont un confort, pas une garantie.** Le serveur revalide tout,
et les règles vivent dans le domaine (phase 6). Un panneau rouvert sur un
message d'erreur venu du serveur est le cas normal, pas l'anomalie.

## Pas de surlignage de la ligne neuve

La maquette en montre un. **Il est abandonné**, et c'est une décision de cette
phase.

Le relevé va du plus ancien au plus récent : la ligne écrite arrive en bas,
loin du panneau qui l'a produite, d'où l'idée de la désigner. Mais le rendu ne
sait pas qu'un mouvement vient d'être écrit — il faudrait descendre un booléen
sur trois niveaux (`rendre_fiche` → `contenu_de_l_onglet` → `rendre_onglet`),
puis un drapeau sur la dernière ligne du view model, le tout faux partout
ailleurs. Trois signatures élargies pour une animation de deux secondes.

Le solde du bandeau, lui, change à vue — et c'est le chiffre qu'on regarde
après avoir cliqué « Appliquer ».

## Responsivité

Desktop-first, breakpoint unique à 768 px. Sous ce seuil : le bouton passe en
pleine largeur, le formulaire en colonne, le pied du panneau en colonne. Le
reste de l'écran — en-tête, onglets, colonne « Solde après » qui disparaît — est
déjà tenu par la phase 2 de l'onglet Trésorerie et par `app-layout.html`.

## CSS

Les règles du bouton, du panneau et de la ligne d'ajustement rejoignent
`pages/team-treasury.css`, la feuille existante de cet écran, portée par
`.team-treasury`. Pas de feuille neuve : elle n'aurait pas de racine à elle, et
l'axe 14 de `check-arch` exigerait de l'inscrire au bundle pour un fragment de
la même page.

## Règles métier

Fixées en phase 1, et que le front donne à voir :

1. Le bouton n'apparaît que pour `SpacePermissions::is_admin()`, **qui exclut le
   propriétaire de l'équipe**. Le relevé, lui, reste public.
2. Le sens porte le signe ; le montant saisi est positif.
3. Le montant est un **multiple de 5 kPo** — comme tous les prix du jeu.
4. Il est **plafonné à 500 kPo**. C'est un garde-fou de frappe : 1200 au lieu
   de 120.
5. Un retrait supérieur au solde est **refusé**. `TreasuryMovement::debit()`
   écrête déjà, mais écrêter en silence écrirait un montant que personne n'a
   voulu.
6. Le **motif est obligatoire**. Il se lira dans un relevé public, des mois plus
   tard, mêlé à des mouvements qui s'expliquent seuls. C'est la seule règle qui
   diverge des points manuels, où le motif est facultatif — eux vivent sur une
   page de gestion qui les montre ensemble.
7. L'ajustement est **public**, avec le nom de son auteur.
8. Il est possible **à tout moment**, sans contrainte de phase de jeu.
9. Un crédit grossit « Encaissé », un débit « Dépensé ». Pas de cinquième terme
   dans l'équation du bandeau : il l'élargirait pour un cas rare.
10. **Rien ne se défait** — un ajustement de sens opposé corrige une erreur.

Le plafond se vérifie **avant** le solde : 1200 au lieu de 120 est une faute de
frappe, et le dire ainsi vaut mieux que « le solde ne couvre pas ce retrait ».

### Rien de neuf à cette étape

Question posée le 2026-10-01 : la phase 2 n'ajoute aucune règle métier. Elle
décide d'une composition et d'un mécanisme de réponse, pas de ce qui est permis.
