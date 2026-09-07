# Phase 7 — Effets de bord : l'encart du coach connecté

**Entrée** : `05-use-cases.md` validé, `06-domaine.md` réduit à R28.

## Ce que l'existant impose, et qui a été vérifié

| Fait | Conséquence |
|---|---|
| `io/web/widgets/` existe déjà — trois widgets non-admin | le nouveau s'y range, pas ailleurs |
| `templates/widgets/` existe, deux gabarits | idem |
| Le conteneur `detail-right` ouvre sur `<div class="tabs">` | l'encart se pose **juste avant**, dans le même conteneur |
| Les tables et le port existent (unité 1) | **aucune migration** |
| `verifier_saison_de_la_competition` est privée dans `admin_page.rs` | elle déménage — cf. ci-dessous |

## 1. Persistance

Une requête, une méthode de port.

```rust
async fn list_open_surveys_for_season(&self, season_id: &str, maintenant: &str)
    -> Result<Vec<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

`sql/presences/list_open_surveys_for_season.sql` — une requête pour la saison,
jamais une par journée, et **elle rend des agrégats** : l'encart a besoin de
`statut()`, de la réponse de chaque équipe et de son horodatage, trois questions
du domaine qu'un DTO aurait aplaties.

**Le filtre « ouverte » est dans le SQL *et* dans le domaine, et c'est assumé.**
Le SQL élague — échéance passée, `close_le` renseigné — puis `statut()` décide.
La référence reste le domaine ; un désaccord ne produit qu'une campagne chargée
pour rien, jamais une campagne affichée à tort. L'alternative, charger toutes les
campagnes de la saison à chaque affichage de page, ferait payer les visites qui
ne concernent personne.

Test d'intégration : une campagne échue **de la veille** n'est pas rendue, et
une campagne close par décision non plus, alors que son échéance est à venir.

## 2. Événements — aucun

Ni domain event, ni app event, ni événement DOM (phase 2). L'action rend le
fragment ; il n'y a qu'un consommateur, lui-même.

## 3. Les deux handlers, et le déménagement qu'ils imposent

```rust
// io/web/widgets/presence_call_widget.rs
pub async fn get_presence_call(auth_session: AuthSession, Path(…), State(state)) -> Response
pub async fn post_presence_call_answer(auth_session: AuthSession, Path(…), State(state), Json(body)) -> Response
```

**Un fichier pour les deux.** L'onglet d'administration en séparait trois parce
qu'il portait deux widgets et neuf actions ; ici l'action rend le même fragment
que le GET, et les séparer ferait deux fichiers dont l'un compterait vingt
lignes.

### La garde du non-admin

Les deux appellent `saison_de_la_competition(season_id, competition_id, state)`
— **et rien d'autre** : n'importe quel coach voit l'encart, et il ne montre que
ses propres équipes.

Cette fonction est aujourd'hui `verifier_saison_de_la_competition`, privée dans
`admin_page.rs`. Elle **déménage dans `admin_scope.rs`**, auprès de ses quatre
sœurs — `journee_de_la_saison`, `appariement_de_la_saison`,
`groupe_de_la_saison`, `equipe_de_la_saison` — qui sont toutes des vérifications
« cette ressource appartient-elle à ce parent ».

**Par copier-coller exact**, imports adaptés, jamais réécrite : son `404`
volontaire porte un motif — *répondre 403 confirmerait l'existence de la saison
à qui essaie des identifiants* — qu'une réécriture perdrait. `require_admin_access`
l'importe désormais au lieu de la contenir.

Le nom `admin_scope` devient discutable pour un module qu'un handler non-admin
importe. C'est un défaut de **nom**, pas de conception, et le corriger touche une
dizaine d'imports : **carte à part, hors épic**.

### Ce que rend chaque issue

| Issue | Réponse |
|---|---|
| succès | l'encart réaffiché, `200` |
| `SurveyClosedForCoach`, `RoundFrozenByReport` | une carte explicative, `200`, sans boutons |
| `TeamNotOwnedByCoach`, `TeamNotInSurvey` | `403` — htmx ne remplace rien |
| panne | `500` + `tracing::error!` |

La carte explicative n'est pas une politesse : sans elle, un clic arrivé après la
clôture réafficherait un fragment **vide**, l'encart disparaîtrait sous le
curseur, et le coach lirait ce vide comme un succès.

## 4. Gabarit, feuille, insertion

| Fichier | Contenu |
|---|---|
| `templates/widgets/presence-call.html` | l'encart, ses deux mises en forme, et la carte de refus |
| `templates/competition-detail.html` | **modifié** — un conteneur `hx-get`, juste avant `<div class="tabs">` |
| `assets/static/css/pages/presence-call.css` | + inscription dans `FEUILLES_APP` |

```html
<div hx-get="{{ app_routes.competitions.presence_call(space_id, competition_id, season_id) }}"
     hx-trigger="load"
     hx-swap="outerHTML"></div>
```

`hx-swap="outerHTML"` : le conteneur **disparaît** avec sa réponse. C'est ce qui
permet à `campagnes` vide de ne laisser aucune trace — pas de `<div>` de zéro
hauteur, donc pas de marge orpheline, donc un espacement de page identique selon
qu'un sondage est ouvert ou non.

`hx-disinherit="*"` sur la racine du fragment. Portée `.presence-call`, aucun
`style="…"`, tokens `--p0` à `--p5`, breakpoint `768px`. Le vert des boutons
« Je serai là » est assombri comme dans la maquette — `--green` porte 3,3:1 sous
du blanc, et **le token n'est pas touché**.

## 5. Tests E2E

`tests/e2e/test_competition_encart_presence.py`.

| Scénario | Ce qu'il éprouve |
|---|---|
| coach avec une équipe, campagne ouverte | l'encart apparaît, deux boutons |
| il répond, la page ne bouge pas | l'onglet ouvert reste ouvert, le classement n'est pas rechargé |
| il change d'avis | la bascule, sans rechargement de page |
| coach avec **deux** équipes | deux lignes, une réponse par équipe (R1) |
| coach sans équipe engagée | **rien du tout** — pas d'encart vide, pas de marge |
| campagne close entre l'affichage et le clic | la carte explicative, pas un fragment vide |
| après le tirage, il se décommande | R30 — la mention, et **aucun adversaire annoncé** |
| deux campagnes ouvertes | deux cartes, la plus proche échéance en premier |

Le sixième est celui qui vaut le plus : il éprouve une course que personne ne
rencontre en développement, et dont le mode d'échec — un encart qui disparaît
silencieusement — ressemble à un succès.

**`cliquer_quand_cable` sur les boutons de l'encart** : il est injecté par htmx,
donc la fenêtre où il est peint mais pas encore câblé s'y présente.

`tests/impact-map.toml` est mis à jour dans le même commit.

## 6. Ce que cette phase ne change pas

**Le handler de la page de détail n'est pas touché.**
`CompetitionDetailTemplate` ne gagne aucun champ, et les six onglets ne
connaissent pas l'encart. C'est ce qui rend la fonction retirable : supprimer le
conteneur `hx-get` suffit à faire disparaître l'encart, sans toucher à une page
que six onglets partagent.
