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

- [x] `public_router()`, mergé dans `main.rs` avec son commentaire
- [x] **Le test du routeur, sans cookie** — le verrou de cette carte
- [x] ~~`PresenceLinkPath`~~ → **deux routes littérales**, le `404` vient du routeur
- [x] Le handler, découpé : convertir, charger, enregistrer, rendre
- [x] Quatre gabarits — trois pages plus la mise en page —, la feuille au bundle
- [x] `PresenceUnknownTemplate` ne porte que le chemin du bundle (R26)
- [x] `make lint`, `make check-arch`, `make test` — 1894/1894

## Deux routes littérales, et pas de verbe à parser

La carte prévoyait un `PresenceLinkPath` et un `404` « sur un troisième verbe ».
Deux chemins littéraux font mieux :

```rust
.route(path::PRESENCE_OUI, get(presence_oui))
.route(path::PRESENCE_NON, get(presence_non))
```

`/presence/xxx/peut-etre` rend `404` **par le routeur**, sans une ligne de code :
pas de verbe à extraire, pas de `match` à écrire, pas de branche à tester. La
contrainte est portée par la déclaration, il n'y a rien à oublier.

## Le harnais ne savait pas requêter sans session

`Harnais::connecte_en_tant_que` pose toujours un cookie, et un test qui l'emploie
ne peut pas distinguer « la route est publique » de « la route accepte ma
session ». D'où `sans_session` et `get_anonyme`, qui n'émet **pas** l'en-tête
`cookie` — et non un cookie vide : `tower-sessions` traite les deux différemment,
et une chaîne vide prouverait seulement qu'une session illisible est refusée.

## Le vrai verrou de R26 est structurel

Quatre tests, dont un qui ne passe par aucune requête :
`PresenceUnknownTemplate` n'a **qu'un champ**, le chemin du bundle CSS. Un gabarit
qui ne reçoit ni équipe, ni journée, ni compétition ne peut rien en laisser
filtrer, quelle que soit la prose qu'on y écrira demain — et lui ajouter un champ
casse la compilation du test.

C'est plus fort qu'une assertion textuelle, et la suite le montre.

## Deux erreurs de rédaction

**« clos » matchait `onclose`** dans le script de rechargement à chaud injecté en
debug : le test échouait sur une page juste. C'est le même défaut que le
`not_to_contain_text("01")` de la carte 522, à quelques heures d'écart — une
sous-chaîne trop courte qui attrape autre chose. Les assertions portent désormais
sur des **phrases**, et le motif est écrit dans le test.

**La portée CSS** : la racine était `.presence-page` dans un fichier
`presence-response.css`. L'axe 15 l'a refusé — le nom du fichier *est* le
sélecteur de portée.

## Le risque résiduel de R4, écrit plutôt que tu

Un `GET` qui enregistre est à la portée d'une machine : SafeLinks et certains
antivirus visitent les URL. R4 assume ce risque et pose comme parade le bouton
opposé sur la page, non un second clic — qui renierait la promesse « un clic
suffit ».

Ce que la parade ne couvre pas, et qui est noté dans le handler : une
prévisualisation automatique peut enregistrer « oui » sans que le coach ait
cliqué. Le bouton opposé le rattrape s'il ouvre la page ; il ne le rattrape pas
s'il ne l'ouvre jamais. Décision produit déjà prise, non rouverte.
