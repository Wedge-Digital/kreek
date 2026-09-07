# Phase 3 — Architecture back : la réponse du coach

**Entrée** : la phase 1 (l'e-mail à deux boutons, la page d'atterrissage à
quatre états) et l'unité `onglet-presences` complète, qui crée la campagne que
celle-ci lit.

**Pas de phase 2, et ce n'est pas un oubli.** Un e-mail n'a pas d'architecture
front, et la page de réponse est un rendu serveur complet : ni HTMX, ni widget,
ni événement DOM.

## Ce que l'existant impose, et qui a été vérifié

| Fait | Conséquence |
|---|---|
| Les neuf routeurs de BC sont dans `protected`, sous `require_auth` ; seul `auth::router::router()` est mergé dans `auth_app` (`main.rs`) | `competitions` expose un **second routeur, public** |
| `reset_token.rs` : un `SUlid` opaque, `/auth/password/update/{reset_token}`, `Path(String)` | le patron du jeton existe — aucune signature à inventer |
| `IEmailService::send(to, subject, html)`, quatre gabarits `emails/fr_FR/*.html` | un cinquième et un sixième s'y insèrent sans réécrire une ligne |
| `DeliveryKey { notification_type, season_id, round_id, target_date, coach_id }`, index unique par destinataire | la relance rattrape exactement ceux qui n'ont rien reçu |
| La maquette d'e-mail porte `yes_url_1`/`no_url_1`, `yes_url_2`/`no_url_2` | R1 est déjà dans le gabarit : **un e-mail, N paires de boutons** |

## Le lien de réponse

```
/presence/{token}/oui
/presence/{token}/non
```

**Un seul jeton par réponse, et c'est le chemin qui porte le sens.**

Le README écrivait que le bouton « finalement je ne pourrai pas » est *« un lien
vers l'autre jeton »*. C'était une formulation de phase 1, et elle contredit tout
ce qui a été conçu depuis : `Reponse` porte **un** `token`, la table a **un**
`UNIQUE (token)`, et l'agrégat expose `reponse_par_jeton(token)`.

Deux jetons auraient voulu dire deux colonnes — ou une table de jetons — pour
distinguer deux liens qui désignent la même réponse. Et cela n'aurait rien
protégé : les deux arrivent dans le même e-mail, qui tient l'un tient l'autre.

**Le chemin est court parce qu'il voyage dans un e-mail** : pas de `space_id`,
pas de `competition_id`, pas de `season_id`. Le jeton les résout tous — c'est
d'ailleurs la seule chose qu'il ait à faire.

`{oui|non}` est un segment de chemin converti par le smart constructor de
`Venue`, comme n'importe quel DTO plat : un troisième mot rend `404`, pas un
refus métier. Deux routes littérales auraient évité la conversion, au prix de
deux handlers identiques à un argument près.

## Le routeur public

```rust
// src/app/competitions/router.rs
pub fn public_router() -> Router<AppState>
```

Mergé dans `main.rs` **au même niveau qu'`auth`**, hors de `protected`.

C'est structurellement neuf : aucun BC hors `auth` n'avait de route publique, et
`protected` porte trois `route_layer` — `require_auth`, `bypass_auth`,
`space_scope` — dont ce routeur doit sortir. Il n'a pas de session à lire, et
son chemin ne porte aucun `space_id` que `space_scope` pourrait résoudre.

**Le commentaire à poser dans `main.rs` compte autant que la ligne.** L'erreur
qui guette n'est pas d'écrire ce routeur, c'est de le ranger dans `protected` par
réflexe le jour où l'on ajoute une route à côté — le sondage cesserait alors de
répondre aux liens déjà partis, sans qu'aucun test unitaire ne le voie.

## La page d'atterrissage

`io/web/public/presence_response.rs`, quatre états rendus par le serveur :

| État | Quand |
|---|---|
| présence confirmée | la réponse est enregistrée à `Presente` |
| absence enregistrée | à `Absente` |
| sondage clos | le jeton est valide, la campagne est close (R7) |
| lien inconnu | aucun jeton ne correspond |

**Un gabarit public à `competitions`, pas l'`auth-layout` d'`auth`.** La phase 1
disait « la page reprend `auth-layout` » — c'était une intention visuelle, et le
mécanisme se tranche ici.

`auth` est un **BC extractible** : un `{% extends %}` depuis `competitions` vers
son gabarit créerait précisément l'adhérence que ce statut proscrit. Et rien ne
le signalerait — `askama.toml` déclare les onze dossiers de templates dans un
**espace de noms unique**, donc le chemin résout sans qu'on voie qu'il traverse
une frontière.

Le coût du choix retenu est une trentaine de lignes de mise en page dupliquées,
et sa feuille inscrite au bundle. Les deux autres voies coûtaient davantage :
sortir `auth-layout` chez l'hôte obligerait `auth` à en garder une copie — même
duplication, ailleurs — et l'`extends` direct romprait le statut extractible pour
économiser ces trente lignes.

**Aucun `<link>` dans le gabarit** : la feuille passe par `css_bundle.rs`, comme
toutes les autres. L'exception d'`auth` — qui charge encore les siennes — ne
s'étend pas à une page de `competitions`.

## R4, à l'écran

Le clic enregistre, et **la page affiche aussitôt le bouton opposé**. Ce n'est
pas un ornement : un `GET` qui enregistre est à la portée d'une machine —
Outlook SafeLinks et certains antivirus visitent les URL pour les inspecter.

La parade n'est pas d'ajouter un second clic, qui renierait la promesse « un clic
suffit », mais de rendre le geste réversible sur la page même. Un préfetch pose
une présence que le coach voit et corrige en un clic — et il pouvait de toute
façon changer d'avis jusqu'à la clôture.

C'est aussi ce que le jeton unique permet : le bouton opposé est **le même
jeton, l'autre verbe**.

## L'e-mail

`emails/fr_FR/competition_presence_survey.html` et son jumeau de relance, sur le
patron des quatre existants — dégradé `#003049 → #555770`, logo en URL absolue
servi en 200×81, `width`/`height` en attributs HTML pour Outlook, tout le style
en ligne.

```rust
pub struct PresenceSurveyEmail {
    pub app_url: String,
    pub coach_name: String,
    pub competition_name: String,
    pub competition_url: String,
    pub round_name: String,
    pub deadline: String,
    pub equipes: Vec<EquipeLigneVm>,   // R1 — une paire de boutons par équipe
}

pub struct EquipeLigneVm { pub team_name: String, pub yes_url: String, pub no_url: String }
```

**Un seul message par coach**, portant autant de paires de boutons que d'équipes
(R1). Un e-mail par équipe multiplierait les messages pour la même soirée, et le
coach ne saurait pas lequel il a déjà traité.

### Deux `NotificationType`, pas un

```rust
PresenceSurvey    => "presence_survey"
PresenceReminder  => "presence_reminder"
```

`DeliveryKey` portant déjà `target_date`, une seule variante à deux dates aurait
mécaniquement suffi. Deux variantes valent mieux pour deux raisons : le journal
dit alors **ce qui** est parti et pas seulement quand, et une relance envoyée le
jour même de l'ouverture cesse d'être bloquée par une clé qu'elle partagerait
avec l'envoi initial.

Les valeurs stockées sont écrites à la main et figées par un test, comme les
quatre autres : un renommage de variante réarmerait sinon en silence toutes les
notifications déjà envoyées.

## `ISurveyMailer` — son implémentation

`io/email/survey_mailer.rs`, dans la couche IO du BC. Le trait est déclaré par la
couche applicative (carte 515) ; c'est ici qu'il rencontre `IEmailService`, le
journal `notification_deliveries` et `app_url`.

Le protocole est celui du journal existant : **`claim` puis `confirm`, jamais
l'inverse.** `claim` réserve le créneau avant l'envoi, l'index unique départage
deux appels parallèles, et zéro ligne rendue signifie « déjà envoyé ». Si l'envoi
échoue, la ligne reste avec `sent_at` à `NULL` — un échec constaté, que R20 veut
journalisé et qu'une relance rattrape.

Cette implémentation **remplace celle qui journalisait sans envoyer**, livrée par
la carte 515 pour que l'onglet soit utilisable avant cette unité.

## Le port gagne `find_by_token`

```rust
async fn find_by_token(&self, token: &str)
    -> Result<Option<PresenceSurvey>, PresenceSurveyRepositoryError>;
```

Reportée ici depuis l'unité 1, comme décidé : c'est cette unité qui sait ce que sa
route publique charge. L'ajout au trait est un ajout, pas une reprise.

La requête joint la campagne à la réponse par le jeton. **Aucune colonne
d'expiration à consulter** : R7 refuse au jeton toute échéance propre, sa
validité se lit sur l'état de la campagne.

## Plan de fichiers

| Fichier | Contenu |
|---|---|
| `router.rs` | **modifié** — `public_router()` |
| `src/main.rs` | **modifié** — mergé à côté d'`auth`, avec son commentaire |
| `io/web/public/presence_response.rs` | le handler des quatre états |
| `io/web/templates/public/presence-response.html` | le gabarit, mise en page comprise |
| `assets/static/css/pages/presence-response.css` | + son inscription au bundle |
| `io/email/notification_emails.rs` | **modifié** — deux structs de plus |
| `assets/templates/emails/fr_FR/competition_presence_survey.html` | et son jumeau de relance — les gabarits d'e-mail vivent chez l'hôte, pas dans le BC |
| `io/email/survey_mailer.rs` | l'implémentation d'`ISurveyMailer` |
| `domain/notification_delivery.rs` | **modifié** — deux variantes, deux valeurs figées |
| `domain/presence_survey_repository_port.rs` | **modifié** — `find_by_token` |
| `io/repository/sql/presences/find_survey_by_token.sql` | la requête |

## Ce que cette unité n'ajoute pas

**Aucun use case.** La réponse par jeton appelle `record_answer` avec
`Repondant::Coach` — le use case existe, et R1 porte sur l'équipe, jamais sur le
coach. Un use case propre à la route publique aurait fait un second endroit où
tenir R13 et R19.

**Aucun domain event, aucun app event.** Personne hors du BC n'a à savoir qu'un
coach a répondu.

**Aucune méthode domaine.** L'agrégat a été conçu d'un bloc en phase 6 pour les
trois unités ; `reponse_par_jeton` et `enregistrer` l'attendaient déjà.

## Règle métier apparue en phase 3

### R25 — Le jeton désigne la réponse, pas le sens de la réponse

Apparue en phase 3, en tranchant la forme du lien.

Un jeton identifie **quelle équipe répond dans quelle campagne**. Ce que le coach
répond est porté par le chemin, jamais par le jeton — d'où un seul jeton par
réponse, et deux liens qui n'en diffèrent que par le dernier segment.

**Conséquence à assumer** : le lien reste utilisable pour changer d'avis, autant
de fois qu'on veut, jusqu'à la clôture. Ce n'est pas une faiblesse, c'est R4 —
un lien qui ne servirait qu'une fois rendrait irréversible une réponse qu'un
antivirus a pu poser tout seul.

**Conséquence sur la clôture** : c'est la campagne qui ferme, pas le lien (R7).
Rouvrir réarme les anciens liens sans rien réémettre, et cette phase n'y change
rien.
