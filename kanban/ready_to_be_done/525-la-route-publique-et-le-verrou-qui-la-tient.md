# La route publique, et le verrou qui la tient

**Priorité : haute — c'est la promesse « un clic suffit »**
**Épic :** E16 — Sondage de présence
**Dépend de :** 523, 524, et 516 pour `record_answer`
**Fichiers :** `src/app/competitions/router.rs`, `src/main.rs`,
`src/app/competitions/io/web/public/presence_response.rs`,
`.../templates/public/presence-{response,closed,unknown}.html`,
`assets/static/css/pages/presence-response.css`, `src/web/css_bundle.rs`,
`src/web/tests/` (le test du routeur)

## L'objectif

Le coach clique dans sa boîte mail, sans connexion, et sa réponse est
enregistrée.

```
/presence/{token}/oui
/presence/{token}/non
```

Un seul jeton, le sens dans le chemin (R25). Le chemin est court parce qu'il
voyage dans un e-mail : ni `space_id`, ni `competition_id`, ni `season_id` — le
jeton les résout tous.

## Ce qui est structurellement neuf

**Aucun BC hors `auth` n'a de route publique.** Les neuf routeurs de BC sont dans
`protected`, sous `require_auth`, `bypass_auth` et `space_scope`.
`competitions` expose donc un second routeur, mergé dans `main.rs` **à côté
d'`auth`** :

```rust
let auth_app = Router::new()
    .merge(app::auth::router::router())
    .merge(app::competitions::router::public_router())   // <- ici
    .merge(protected);
```

Le handler est le seul de `competitions` sans `AuthSession` et sans
`require_admin_access`.

### Le verrou est un test, pas un commentaire

Un commentaire n'a jamais arrêté personne, et l'erreur — ranger ce routeur dans
`protected` en ajoutant une route à côté — **ne casserait aucun test existant**.
Les liens déjà partis cesseraient de répondre, et on l'apprendrait par un coach.

Le test monte **le routeur de production**, demande `/presence/{token}/oui`
**sans cookie de session**, et attend autre chose qu'une redirection vers
`/auth/login`. Même patron que `src/web/tests/test_cookie_de_session.rs`, et pour
la même raison : il lit ce que le routeur fait, pas ce qu'on croit lui avoir dit.

## Les trois états

| Issue | Page |
|---|---|
| `Ok(_)` | présence confirmée / absence enregistrée — **le bouton opposé juste dessous** (R4) |
| jeton introuvable | lien inconnu — le VM ne porte **aucun** champ (R26) |
| `SurveyClosedForCoach` (R21) | sondage clos, motif « échéance » ou « décision » |
| `RoundFrozenByReport` (R13) | sondage clos, motif « journée déjà jouée » (R27) |

**Trois gabarits pour quatre états** : « confirmée » et « absente » ne diffèrent
que par leurs mots — même encadré, même récapitulatif, même bouton opposé. Les
séparer aurait dupliqué un markup qui doit rester en phase.

Le use case reçoit **`Repondant::Jeton`** (R28) : ici c'est le lien qui
autorise, pas une session — il n'y a aucun propriétaire d'équipe à confronter.

**R19 est inatteignable par ce chemin** et n'a donc pas d'écran : le jeton *est*
la désignation de la réponse, l'équipe est dans la campagne par construction. Une
page pour ce cas serait un écran que personne ne peut atteindre, et que personne
ne vérifierait jamais.

**R4 en un mot** : un `GET` qui enregistre est à la portée d'une machine —
SafeLinks et certains antivirus visitent les URL pour les inspecter. La parade
n'est pas un second clic, qui renierait la promesse, mais le bouton opposé sur la
page même.

## Le gabarit et la feuille

**Pas d'`{% extends %}` vers `auth-layout`.** `auth` est un BC extractible ; la
page porte donc sa propre mise en page, dans le dossier de templates de
`competitions`. Trente lignes dupliquées contre une dépendance qu'aucun `grep` ne
verrait — `askama.toml` déclarant les onze dossiers dans un espace de noms unique.

La page charge le bundle applicatif par
`{{ crate::web::css_bundle::chemin_app() }}` : elle porte son propre `<link>`
puisqu'elle n'étend pas `app-layout`, mais pointe le même fichier. Il n'existe
qu'un bundle, et en créer un second pour une page publique coûterait une liste,
une construction et un cas d'axe 14 de plus, pour une requête mise en cache.

`presence-response.css` s'inscrit dans `FEUILLES_APP` au rang « pages ».

## Checklist

- [ ] `public_router()`, mergé dans `main.rs` avec son commentaire
- [ ] **Le test du routeur, sans cookie** — le verrou de cette carte
- [ ] `PresenceLinkPath`, la conversion par smart constructors, `404` sur un
      troisième verbe
- [ ] Le handler, découpé : convertir, charger, enregistrer, rendre
- [ ] Les trois gabarits, leur mise en page, la feuille inscrite au bundle
- [ ] `PresenceUnknownTemplate` ne porte qu'`app_url` (R26)
- [ ] `make lint`, `make check-arch`, `make test`
