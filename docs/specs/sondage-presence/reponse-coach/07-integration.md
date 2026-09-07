# Phase 7 — Effets de bord : la réponse du coach

**Entrée** : `05-use-cases.md` validé, `06-domaine.md` sans objet. Reste à
nommer les requêtes, le câblage, les gabarits et les scénarios.

## Ce que l'existant impose, et qui a été vérifié

| Fait | Conséquence |
|---|---|
| Les gabarits d'e-mail vivent dans `assets/templates/emails/fr_FR/`, pas dans le BC | les deux nouveaux y vont ; la phase 3 disait le contraire, elle est corrigée |
| Il n'existe **qu'un** bundle CSS, `"app"`, exposé par `chemin_app()` | la page publique le charge tel quel — cf. ci-dessous |
| `find_team_names(&[String])` existe sur `ITeamInfoPort` | rien à ajouter au port inter-BC |
| Les tables de campagne et de réponses existent (unité 1, carte 510) | **aucune migration** |

## 1. Persistance

Deux requêtes de lecture, sous `sql/presences/` comme les cinq de l'unité 1.

| Fichier | Rôle |
|---|---|
| `find_survey_by_token.sql` | la campagne dont une réponse porte ce jeton, avec toutes ses réponses |
| `find_landing_labels.sql` | les libellés de journée, saison, compétition et espace |

`find_by_token` s'ajoute au port — l'ajout attendu depuis l'unité 1, où il avait
été délibérément reporté.

**La requête ne consulte aucune colonne d'expiration**, parce qu'il n'y en a pas :
R7 refuse au jeton toute échéance propre, et sa validité se lit sur l'état de la
campagne, calculé par `statut_de` (R23). Le jeton est un pointeur, rien de plus.

**`find_landing_labels.sql` ne joint que des tables de `competitions`** — saisons,
compétitions, journées. Le nom de l'équipe vient du port, jamais d'une jointure :
c'est la souveraineté des données entre BCs, et une jointure vers `teams` serait
l'exacte violation que la règle nomme.

Test d'intégration : `find_by_token` sur un jeton inconnu rend `None` et non une
erreur — c'est cette distinction qui fait la page « lien inconnu » plutôt qu'un
`500`.

## 2. Événements — aucun

Ni domain event, ni app event, ni listener. Personne hors du BC n'a à savoir
qu'un coach a répondu, et R14 garantit que le classement n'est pas concerné.

## 3. Le handler, et le câblage qui va avec

```rust
pub async fn get_presence_response(
    Path(lien): Path<PresenceLinkPath>,
    State(state): State<AppState>,
) -> Response
```

**Pas d'`AuthSession`, et c'est tout le sujet de cette unité.** C'est le seul
handler de `competitions` qui n'en reçoit pas, et le seul qui n'appelle pas
`require_admin_access`.

Il enchaîne : convertir le chemin, charger par jeton, appeler `record_answer`,
hydrater, choisir l'un des trois gabarits selon le tableau de la phase 5. Quatre
fonctions nommées, pas un corps de quarante lignes.

### Le câblage, et le verrou qui le tient

```rust
// main.rs — à côté d'auth, jamais dans `protected`
let auth_app = Router::new()
    .merge(app::auth::router::router())
    .merge(app::competitions::router::public_router())   // ← ici
    .merge(protected);
```

La phase 3 disait que le commentaire compte autant que la ligne. **C'est vrai et
insuffisant** : un commentaire n'a jamais arrêté personne, et l'erreur — ranger
ce routeur dans `protected` en ajoutant une route à côté — ne casserait aucun
test existant. Les liens déjà partis cesseraient simplement de répondre, et on
l'apprendrait par un coach.

Le verrou est donc un test, sur le patron de
`src/web/tests/test_cookie_de_session.rs` : il monte **le routeur de production**,
demande `/presence/{token}/oui` **sans cookie de session**, et attend autre chose
qu'une redirection vers `/auth/login`.

Ce test lit ce que le routeur fait, pas ce qu'on croit lui avoir dit — la même
raison qui a fait écrire celui du cookie.

## 4. Gabarits, feuille, e-mails

| Fichier | Contenu |
|---|---|
| `io/web/templates/public/presence-response.html` | présence confirmée et absence enregistrée |
| `io/web/templates/public/presence-closed.html` | sondage clos, ses trois motifs (R27) |
| `io/web/templates/public/presence-unknown.html` | lien inconnu — aucune donnée (R26) |
| `assets/templates/emails/fr_FR/competition_presence_survey.html` | l'e-mail à N paires de boutons |
| `assets/templates/emails/fr_FR/competition_presence_reminder.html` | la relance |
| `assets/static/css/pages/presence-response.css` | + inscription dans `FEUILLES_APP` |

**Les trois pages chargent le bundle applicatif** par
`{{ crate::web::css_bundle::chemin_app() }}`, comme `app-layout.html`. Il n'existe
qu'un bundle ; en créer un second — plus petit, pour une page publique — voudrait
dire une seconde liste, une seconde construction et un second cas pour l'axe 14,
au bénéfice d'une requête que le navigateur met en cache. La page porte donc son
propre `<link>` parce qu'elle n'étend pas `app-layout`, mais elle pointe le même
fichier.

**La feuille s'inscrit dans `FEUILLES_APP`** au rang « pages », sans quoi l'axe 14
de `check-arch` refuse. Portée `.presence-response`, aucun `style="…"`, tokens
`--p0` à `--p5`, breakpoint `768px`.

Les deux e-mails suivent les quatre existants : dégradé `#003049 → #555770`, logo
en URL absolue servi en 200×81, `width`/`height` en attributs HTML pour Outlook,
tout le style en ligne — un client mail ignore les feuilles externes.

## 5. Tests E2E

`tests/e2e/test_presence_reponse_publique.py`. Le parcours ne peut pas partir
d'un clic dans une boîte mail : il **lit le jeton en base** par `db_helpers.py`,
comme les autres tests qui ont besoin d'un état que l'écran ne montre pas.

| Scénario | Ce qu'il éprouve |
|---|---|
| ouvrir une campagne, lire un jeton, visiter `/oui` | la page confirme, et l'onglet de l'organisateur voit la présence |
| cliquer le bouton opposé | R4 — le même jeton, l'autre verbe, la réponse bascule |
| visiter `/oui` deux fois | idempotent : un préfetch d'antivirus ne casse rien |
| clore la campagne, revisiter | R27 — motif « échéance » ou « décision », et la dernière réponse est affichée |
| publier un rapport, revisiter | R27 — motif « journée déjà jouée », alors que l'échéance est à venir |
| visiter un jeton inventé | R26 — la même page qu'un jeton révoqué, sans un mot de plus |
| visiter sans session, en navigation privée | le verrou du routeur public, vu du navigateur |

Le dernier double le test unitaire du routeur, et ce n'est pas de la redondance :
l'un vérifie que la route n'est pas sous `require_auth`, l'autre qu'un navigateur
sans cookie voit bien la page. Les deux ont échoué séparément ailleurs dans ce
projet.

**Pas de `cliquer_quand_cable` ici** — la page est un rendu serveur complet, sans
HTMX. C'est la seule page du projet dans ce cas, et c'est pour cela que la
fenêtre de câblage ne s'y présente pas.

`tests/impact-map.toml` est mis à jour dans le même commit.

## 6. Ce que cette phase corrige à la précédente

**Le chemin des gabarits d'e-mail.** La phase 3 les plaçait dans
`io/web/templates/emails/fr_FR/` ; ils vivent dans `assets/templates/`, un
dossier de l'hôte déclaré en tête d'`askama.toml`. Les quatre existants y sont
déjà, et un e-mail n'appartient pas plus à un BC que le layout applicatif.
