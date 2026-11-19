# Le montage e2e compte les clics, pas les embauches

**Priorité : haute** — la CI de `demo` échoue une fois sur deux
**Dépend de :** rien · **Sans épic**
**Trouvée par :** la CI, deux échecs alternés sur `test_competition_full_lifecycle`

## Le symptôme

```
TimeoutError: Page.wait_for_selector: Timeout 10000ms exceeded.
  - waiting for locator(".submit-bar") to be visible
page = <Page url='.../team/01M1A195NBD6N7XFKS3CNTQR46/finalize'>
```

L'URL est bien sur `/finalize`, et `.submit-bar` n'y paraît jamais. Reproduit en
local : **un échec sur cinq**.

## La cause

`competition_lifecycle.py`, dans `build_and_submit_team` :

```python
btn.click()
page.wait_for_timeout(150)   # durée fixe
hired += 1                   # incrémenté quoi qu'il arrive
```

Le compteur avance **que le clic ait produit une embauche ou non**. Sous charge,
la requête htmx n'aboutit pas dans les 150 ms — ou le bouton est périmé, sa
ligne ayant été réécrite par le swap précédent (`hx-target="closest tr"`,
`hx-swap="outerHTML"`). L'assertion `hired >= 11` passe alors sur dix joueurs.

Prouvé en base plutôt que déduit, sur l'équipe en échec et ses voisines du même
run :

```
ok     | roster ✓ | 11 joueurs | soumise
ok     | roster ✓ | 11 joueurs | soumise
ok     | roster ✓ | 11 joueurs | soumise
ÉCHEC  | roster ✓ | 10 joueurs | non soumise
```

## Le produit a raison, et il le disait

`finalize_team.rs` refuse en dessous de `MIN_PLAYERS_FOR_SUBMISSION` et rend
« Vous devez recruter au moins 11 joueurs avant de finaliser » via un
`HX-Retarget` vers `#submit-errors`. Le swap réussit, donc `hx-push-url` pousse
`/finalize` dans la barre d'adresse — mais le contenu reste celui de build-team,
avec l'erreur. D'où une URL en `/finalize` sans `.submit-bar`.

**Le message était à l'écran, et le test ne l'a pas lu.** Il a attendu un
sélecteur pendant dix secondes à côté d'une phrase qui disait la cause.

## Ce que ce n'est pas

**Pas la taille de la base de développement.** C'était le diagnostic donné lors
de la carte 476, sur la foi d'une mesure juste — 22 640 matchs projetés — et
d'une inférence fausse. La CI reproduit l'échec sur une base **neuve**, deux
fois sur quatre exécutions.

## Trois copies du même défaut

| Fichier | Délai fixe |
|---|---|
| `competition_lifecycle.py` — `build_and_submit_team` | 150 ms ← celui qui casse |
| `test_special_rule_selector.py` — `_recruter` | 200 ms |
| `test_build_and_finalize_team.py` | 300 ms |

Les deux autres ne tiennent que par un délai plus généreux. Les corriger
maintenant évite de revenir deux fois.

## La correction

Un helper partagé qui **attend la postcondition au lieu de parier sur une
durée** — la doctrine que `_attendre_finalisation` applique déjà dans ce même
fichier, avec un docstring qui dit pourquoi.

Il compte ce que le serveur a réellement enregistré :

```sql
SELECT jsonb_array_length(COALESCE(state->'hired_players', '[]'::jsonb))
FROM team_roster_selections WHERE id = $1
```

Après chaque clic, on attend que ce nombre monte ; s'il ne monte pas, on
reclique. Aucun `sleep`, et l'échec dit « 10 joueurs sur 11 » au lieu d'expirer
vingt étapes plus loin.

**La base plutôt que le DOM** : la quantité par ligne n'est qu'un `<td>` sans
classe, désignable seulement par son rang parmi treize colonnes. Et c'est
exactement cette donnée-là que `finalize_team` relit pour décider — mesurer la
même chose que le garde qu'on cherche à satisfaire supprime tout écart possible.

## Ce que la carte ne fait pas

**Aucun changement du produit.** Le refus à moins de onze joueurs est correct,
son message est clair, et le `HX-Retarget` fait ce qu'il doit.

On peut discuter que `hx-push-url` pousse une URL dont le contenu n'a pas été
servi — la barre d'adresse ment alors sur ce qu'on regarde. C'est un vrai sujet,
mais il dépasse cette carte et ne touche pas la suite.

## Tests

Le montage **est** le sujet : il se vérifie en le faisant échouer.

- 10 exécutions de `test_competition_full_lifecycle` sans échec (contre 1/5 avant) ;
- le helper, forcé à ne jamais voir monter le compte, échoue en nommant le
  nombre réel plutôt qu'en expirant ailleurs.

## Checklist

- [x] `recruter_joueurs` dans `competition_lifecycle.py`, sans `sleep` fixe
- [x] Les trois appelants passent dessus
- [x] Message d'échec nommant le compte réel
- [x] 10 exécutions vertes de `test_competition_full_lifecycle`
- [x] `make e2e` vert — 333 passés, 7 ignorés

---

# Ce que la réalisation a appris

## La mesure avant la correction

`joueurs_recrutes` a été vérifiée contre les équipes du run qui avait échoué en
local : **10** sur celle qui a fait tomber le test, **11** sur ses trois
voisines. La fonction qui décide de la boucle mesure donc bien ce qu'elle
prétend — c'est ce qui rend le reste concluant.

## Le résultat

| | Avant | Après |
|---|---|---|
| `test_competition_full_lifecycle` en local | 1 échec sur 5 | **0 sur 10** |
| Suite complète | verte, mais rouge une fois sur deux en CI | 333 passés, 7 ignorés |

## Un piège de plus dans la même famille

C'est la cinquième occurrence de la journée du même défaut de conception de
test : **attendre une durée plutôt qu'une postcondition**. Les quatre autres
étaient des clics tombés dans la fenêtre de câblage htmx ; celle-ci est un
compteur qui avance sans preuve.

La forme commune n'est pas « htmx est lent ». C'est qu'un test qui **suppose**
un effet au lieu de **l'observer** échoue loin de sa cause. Ici : vingt étapes
plus loin, sur un sélecteur absent, à côté d'un message d'erreur qui disait
exactement ce qui manquait.

## Ce que le diagnostic précédent avait de faux

La carte 476 attribuait cette instabilité à la taille de la base de
développement. La mesure était juste — 22 640 matchs projetés — mais
l'inférence ne l'était pas : la CI reproduit l'échec sur une base neuve, deux
fois sur quatre exécutions. Une corrélation avait tenu lieu de cause.

## Ce qui reste ouvert, et que la carte ne traite pas

`hx-push-url` pousse `/finalize` dans la barre d'adresse alors que la réponse
servie est une erreur retargée sur `#submit-errors` : le contenu affiché reste
celui de build-team. **L'URL ment sur ce qu'on regarde**, et un rechargement
mènerait ailleurs que ce que l'utilisateur voit.

Sans conséquence connue aujourd'hui — le coach corrige son effectif et
continue — mais c'est un vrai sujet, distinct de celui-ci.
