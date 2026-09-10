# Les neuf actions

**Priorité : haute — sans elles l'écran est en lecture seule**
**Épic :** E16 — Sondage de présence
**Dépend de :** 520, et les use cases 515 à 518
**Fichiers :** `src/app/competitions/io/web/admin/presences_actions.rs`,
`src/app/competitions/routes.rs`, `src/app/competitions/router.rs`

## Cette carte déclare et branche ses neuf routes

La 519 n'a posé que les trois routes qu'elle sert. **Les neuf de cette carte lui
appartiennent** — constante de chemin, méthode de génération d'URL, et entrée de
routeur, dans le même commit que leur handler :

```
presences/launch   presences/answer   presences/remind
presences/close    presences/reopen   presences/draw
presences/confirm-draw   presences/undo-draw   presences/repair
```

Une route déclarée sans handler ne compile pas ; une route branchée sur un
handler vide est une porte ouverte sans garde. Les annoncer d'avance aurait
demandé neuf handlers factices, qui passeraient `check-arch` et l'axe 4 sans que
rien ne les appelle.

**Chacune porte `require_admin_access` puis `journee_de_la_saison`**, y compris
sur un POST qui ne rend qu'un fragment : `space_scope` n'a pas de résolveur pour
`round_id`, qui passerait librement (carte 416).

## L'objectif

Neuf handlers POST, et le protocole de réponse qui vaut pour les neuf.

## Le protocole — ce que rend une action

| Cas | Réponse HTTP |
|---|---|
| succès | corps vide + `HX-Trigger: presenceChanged` |
| refus métier (R11, R13, R15, R19, R21, R22) | le fragment de panneau porteur du motif, + `HX-Retarget: #presence-panel` + `HX-Reswap: innerHTML`, **sans** trigger |
| `draw` | le fragment d'aperçu, `HX-Retarget` + `HX-Reswap`, sans trigger |
| corps mal formé, VO invalide | `400` — refus de format, pas refus métier |
| panne | `500` + `tracing::error!` |

Les boutons portent `hx-swap="none"` : le succès ne remplace rien, les deux
widgets se rechargent d'eux-mêmes sur `presenceChanged`. **C'est le serveur qui
redirige le swap** vers le panneau quand il a quelque chose à y dire — mécanisme
htmx standard, pas de JavaScript.

Rendre le fragment *et* déclencher le rechargement peindrait le panneau deux
fois ; rendre un corps vide ne laisserait nulle part où poser le refus.

**`draw` ne déclenche pas `presenceChanged`**, et c'est ce qui le rend possible :
l'aperçu ne persiste rien, un rechargement du panneau le perdrait aussitôt. Le
verbe reste `POST` parce que le tirage est aléatoire — donc non idempotent — et
qu'un `GET` serait rejoué au retour arrière, produisant un tirage différent de
celui qu'on regardait.

## Aucune `alert()`, aucun JSON d'erreur

Écart assumé avec le Calendrier, qui expose `window.handleScheduleActionResponse`
et lève une boîte du navigateur. « Le tirage a produit une revanche parce que
toutes les paires inédites étaient épuisées » n'est pas une alerte : c'est une
explication, elle appartient à l'aperçu qu'elle commente. Une fois cette
explication dans le panneau, les refus y vont aussi — sinon il faudrait trancher
pour chaque nouveau message de quel côté il tombe, et cette frontière dérive.

**Pas de `onclick="fetch(...)"` non plus.** `schedule-round-detail.html` porte des
attributs `onclick` de plus de quatre cents caractères qui reconstruisent un
appel HTMX à la main — c'est précisément ce qui a rendu nécessaire le
`handleScheduleActionResponse` ci-dessus. Les actions sont des `hx-post`
ordinaires.

## Les DTOs d'entrée

Plats, en JSON, convertis par le handler : **DTO plat -> `try_new` -> commande
typée**. Le refus de validation est un `400`, jamais un refus métier.

| DTO | Champs |
|---|---|
| `LaunchBody` | `round_id`, `deadline`, `auto_remind` |
| `AnswerBody` | `round_id`, `team_id`, `presence` |
| `SurveyIdBody` | `round_id` — remind, close, draw, undo-draw |
| `ReopenBody` | `round_id`, `deadline` (R23) |
| `ConfirmDrawBody` | `round_id`, `rencontres`, `exemptee` |
| `RepairBody` | `round_id`, `rencontre`, `exemptee` |
| `PairBody` | `home_team_id`, `away_team_id` |

**La campagne est désignée par `round_id`, jamais par `survey_id`** : R2
garantit une seule campagne vivante par journée, et le client n'a pas à
connaître un identifiant qu'il ne lit nulle part.

**`PairBody` ne porte pas `historique`** : le client n'a pas à renvoyer un motif
que le serveur recalcule de toute façon. Le lui faire porter autoriserait à
mentir sur l'historique d'une rencontre.

## Checklist

- [x] **Dix** handlers et non neuf — cf. ci-dessous —, `require_admin_access` +
      `journee_de_la_saison` sur les dix
- [x] Les sept DTOs, la conversion par smart constructors
- [x] Le protocole : trois helpers — succès, refus, aperçu
- [x] Les boutons des gabarits en `hx-swap="none"`
- [x] Chaque refus métier rend son motif dans le panneau
- [x] `make lint`, `make check-arch`, `make test` — 1877/1877 · 10 cas e2e passés

## Une dixième route : `propose-repair`

Le panneau de défection devait porter un bouton « Réparer », et la première
écriture lui faisait **précalculer** la proposition. Puis le problème est apparu :
`propose_repair` lance `tirer`, qui départage au sort (R17). La proposition
changerait à chaque rechargement du panneau — et le panneau se recharge sur chaque
`presenceChanged`. L'organisateur lirait une proposition, corrigerait une
présence, et en verrait une autre.

C'est exactement le problème que `draw` résout en étant un POST, et la réparation
prend donc la même forme :

```
panneau de défection → POST propose-repair → aperçu → POST repair
   (ce qui ne va pas)   (la proposition, une fois)  (on valide)
```

Une route, un handler et un gabarit de plus, et **le même vocabulaire** pour les
deux propositions — celle du tirage et celle de la réparation — au lieu d'un
chemin qui propose et d'un autre qui devine.

## Le défaut de la 520 que cette carte a fait sortir

`DrawVm::from_domain` mettait `r.home.to_string()` dans le champ affiché —
**l'identifiant de vingt-six caractères, pas le nom**. C'est le défaut de la carte
506, dans un fichier dont le commit affirmait « les noms viennent du roster, jamais
un identifiant brut » : vrai de `journee_appariee`, faux de `DrawVm::from_domain`.

Rien ne pouvait l'attraper en 520 : le panneau d'aperçu n'était pas atteignable,
l'action `draw` n'existant pas. C'est la 521 qui l'a trouvé, en s'en servant.

`DrawRowVm` porte désormais **deux champs séparés** — le nom pour l'écran,
l'identifiant pour que la validation le renvoie. Afficher un identifiant est un
défaut ; renvoyer un nom ne désignerait rien. Test dédié.

## Ce que la réalisation a tranché

**`hx-ext="json-enc"` et non `onclick="fetch(...)"`.** L'extension est chargée
dans `app-layout.html` et `team_creation` s'en sert déjà. C'est ce qui évite les
attributs `onclick` de quatre cents caractères de `schedule-round-detail.html` —
ceux-là mêmes qui ont rendu nécessaire le `handleScheduleActionResponse` que cette
carte refuse.

**Un `ActionsVm` plutôt qu'`app_routes` dans les gabarits.** Les fragments inclus —
carte d'équipe, rencontres — accèdent à `{{ actions.answer }}` sans recevoir les
trois identifiants de chemin qu'ils n'utiliseraient que pour ça.

**`contexte()` rend la journée, pas un booléen.** Les dix handlers n'ont donc pas
à la relire : un contrôle qui coûte une requête de plus se contourne un jour « pour
la performance ». C'est la raison qui a fait rendre l'agrégat à
`journee_de_la_saison`, et elle vaut d'un étage plus haut.

**`UserId` et `CoachId` sont le même type.** Le premier jet écrivait
`CoachId::try_new(&u.id.to_string())` — un aller-retour par une chaîne qui pouvait
échouer pour rien.

**L'échéance proposée est la veille de la journée, ou rien.** Pas de délai
inventé : « dans sept jours » se confondrait avec une date choisie et serait validé
sans regard ; un champ vide se voit.

**Abandonner un aperçu est un `hx-get` sur le panneau.** L'aperçu ne persistant
rien, il n'y a rien à défaire — recharger suffit.

## L'état de la vérification e2e

Les dix cas de `test_presences_tab.py` passent, dont les deux du protocole : un
succès rend un corps vide avec son `HX-Trigger`, un refus rend le panneau avec son
motif, `HX-Retarget` et sans trigger.

**Mais aucun passage complet n'est vert** : treize autres tests sont tombés par
dépassement de délai sous une machine chargée — 554 s contre 424 à 453 pour les
passages verts de la veille. Aucun ne touche les présences, et la carte 535 a
éliminé ce matin la cinquième hypothèse de cette instabilité. C'est un verdict
partiel, et il est dit comme tel.
