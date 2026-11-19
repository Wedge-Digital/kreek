# Hauteur de la tabbar mobile — un seul point de vérité

**Priorité : haute**
**Dépend de :** —
**Contexte :** CSS global (`common.css`, `layout-app.css`) + widgets `recruitment` et `dismissals`

## Problème

`.mobile-tabbar` n'a pas de `height` : sa hauteur découle de son contenu
(icône 20px + libellé + `gap: 2px` + paddings) et vaut **62px** au rendu.
Quatre endroits en dépendent, chacun avec sa propre estimation, aucune
correcte :

| Endroit | Valeur | Écart |
|---|---|---|
| `layout-app.css:175` — `padding-bottom` de `.page-content` | 68px | +6 |
| `layout-app.css:177` — commentaire « tabbar de 57px » | 57px | −5 |
| `recruitment.css:267` — `bottom` du panier fixe | 57px | −5 |
| `dismissals.css:219` — `bottom` du panier fixe | 57px | −5 |

Les paniers ne sont donc pas incohérents avec la valeur *déclarée* : c'est la
tabbar réelle qui a dérivé de sa propre documentation. Rien ne pouvait le
signaler, la hauteur n'étant écrite nulle part.

Conséquence visible sous 768px : les deux paniers fixes chevauchent la tabbar
de 5px. Le total et le bouton de validation passent partiellement sous le menu.

## Portée réelle du défaut

Ce chevauchement fait échouer **cinq tests e2e**, dont quatre par cascade.

Les tests de `test_recruitment_phase.py` partagent un panier serveur et
s'exécutent dans l'ordre du fichier ; chacun nettoie derrière lui.
`test_11` achète une Piétaille (50 kPo) puis échoue sur l'assertion de
géométrie — **avant** d'atteindre le retrait de sa ligne, et son `finally` ne
restaure que le viewport. Le panier garde donc 50 kPo de résidu, et `test_04`,
`test_08` et `test_10` héritent d'une prémisse fausse : ils échouent sur des
montants de trésorerie sans aucun défaut applicatif derrière eux.

`test_dismissals_phase::test_11` échoue sur la même géométrie, sur l'autre
page.

## Solution

Un token unique dans le `:root` de `common.css`, consommé partout, et une
`height` explicite sur `.mobile-tabbar` pour que la valeur déclarée soit
**imposée** et non plus estimée.

**Décision prise sur `env(safe-area-inset-bottom)`** : le token porte la
**hauteur de contenu**, l'inset reste additif et n'apparaît qu'à un seul
endroit — un second token dérivé, que consomment tous ceux qui doivent
dégager la barre :

```css
:root {
  --mobile-tabbar-h: 62px;  /* hauteur de contenu, hors encoche */
  --mobile-tabbar-clearance: calc(var(--mobile-tabbar-h) + env(safe-area-inset-bottom));
}
```

Ainsi la hauteur nue reste lisible et modifiable seule, et aucun consommateur
ne réécrit `env(...)`.

**Attention** : le Chrome de Playwright a un inset nul, donc **aucun test ne
protège ce choix**. Toute évolution du calage sur appareil à encoche se
vérifie à la main.

## Checklist

- [x] `--mobile-tabbar-h` et `--mobile-tabbar-clearance` dans le `:root` de `common.css`
- [x] `height` explicite sur `.mobile-tabbar` (vérifier le `box-sizing` en vigueur)
- [x] `.page-content` (`padding-bottom`) consomme le token de dégagement
- [x] `recruitment.css` et `dismissals.css` consomment le token de dégagement
- [x] Commentaire obsolète « tabbar de 57px » de `layout-app.css` corrigé
- [x] `test_recruitment_phase::test_11` et `test_dismissals_phase::test_11` au vert
- [x] `test_04`, `test_08`, `test_10` de `test_recruitment_phase` au vert **par ricochet, sans modification de test**
- [x] `make check-arch`

## Résultat

`test_recruitment_phase.py` + `test_dismissals_phase.py` : **22 passed**, aucun
test modifié. Les cinq échecs tombent bien avec le seul correctif CSS, ce qui
confirme le diagnostic de cascade.

Sans encoche, le rendu est strictement inchangé : la tabbar fait toujours 62px
avec 4px de retrait haut et bas, et `.page-content` conserve ses 68px de
`padding-bottom` (`--mobile-tabbar-clearance` + `--p05`). Seul le calage des
deux paniers bouge, de 57px à 62px.

## Note

Aucune correction de test n'est attendue : les assertions de trésorerie sont
justes, c'est leur prémisse qui était polluée. Si l'une d'elles restait rouge
après le correctif CSS, ce serait un second défaut, à instruire séparément.

La fragilité de fond — un panier serveur partagé qui rend le fichier dépendant
de l'ordre, où un test qui échoue en contamine trois — dépasse cette carte.
