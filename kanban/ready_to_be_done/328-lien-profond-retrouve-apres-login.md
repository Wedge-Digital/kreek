# `auth` — Un lien partagé survit au passage par la page de connexion

**Priorité : moyenne** — rien n'est cassé, mais tout lien envoyé à l'extérieur
perd sa destination dès que le destinataire n'est pas connecté
**Dépend de :** rien
**Fichiers :** `src/web/middleware/require_auth.rs`,
`src/app/auth/io/web/{get_login.rs,post_login.rs,get_login_success.rs}`,
`src/app/auth/io/web/templates/{auth-login.html,auth-login-success.html}`

## Le problème

On veut pouvoir envoyer un lien profond — la fiche d'une équipe, une
compétition — à quelqu'un d'extérieur. Aujourd'hui, si le destinataire n'est
pas connecté :

1. il clique, arrive sur la page de connexion — **c'est le comportement
   voulu** ;
2. il se connecte, et atterrit sur l'accueil de l'application.

La destination est perdue entre les deux, et le destinataire doit se
débrouiller pour la retrouver — s'il se souvient de ce qu'on lui envoyait.

Le lien se perd en quatre endroits, qu'il faut tous traiter pour que le
parcours tienne de bout en bout :

| Où | Ce qui se passe |
|---|---|
| `require_auth.rs:29` | `Redirect::to(path::LOGIN)` — **l'URL demandée est jetée ici**, c'est la perte d'origine |
| `auth-login.html:10` | le formulaire poste en HTMX ; la destination doit traverser ce POST |
| `post_login.rs:29` | `HX-Redirect` vers `/auth/login/success`, en dur |
| `auth-login-success.html` | le bouton pointe sur `authenticated_home` |

## Ce que `auth` a le droit de savoir

`auth` est un **BC extractible** : il ne connaît aucune route de kreek. Ce
n'est pas un obstacle ici — `AuthContext.authenticated_home` est déjà une
`String` injectée par l'hôte, avec ce commentaire :

> Injecté par l'hôte : `auth` ne connaît pas la page d'accueil de celui qui
> l'héberge — c'est son seul lien sortant, et la condition pour qu'il soit
> extractible.

La destination profonde emprunte exactement le même statut : **une chaîne
opaque**, que `auth` transporte et dont il ne juge que la forme, jamais le
sens. Il ne doit y avoir aucune liste blanche de routes applicatives dans ce
BC — elle le rendrait non extractible, et serait de toute façon à refaire à
chaque nouvelle page.

## La destination ne sort jamais du site — ce qu'il faut prouver

Sans garde, `?next=` transforme notre page de connexion en tremplin de
hameçonnage : le lien porte notre domaine, affiche notre formulaire, et dépose
la victime sur un site tiers une fois le mot de passe saisi. C'est le risque
principal de cette carte, et il se traite par construction.

**Reconstruire plutôt que filtrer.** Ne pas chercher à repérer les valeurs
dangereuses une par une : n'accepter la valeur que si elle s'analyse comme une
référence **relative sans autorité**, puis réémettre soi-même `chemin + requête`
— l'autorité et le fragment éventuels sont abandonnés, jamais recopiés. Une
liste de motifs interdits laisse toujours passer la variante qu'on n'avait pas
prévue ; une reconstruction ne peut produire que ce qu'elle sait écrire.

Les cas à faire échouer, chacun pour une raison différente — ils forment le
tableau du test unitaire :

| Valeur reçue | Ce qu'en fait le navigateur | Verdict |
|---|---|---|
| `/spaces/S/teams/T` | chemin local | **accepté** |
| `https://evil.example/` | autre site | refusé — ne commence pas par `/` |
| `//evil.example/` | URL protocole-relative → autre site | refusé — commence par `//` |
| `/\evil.example/` | `\` normalisé en `/` → `//evil.example` | refusé — aucune contre-oblique |
| `/%2F%2Fevil.example` | décodé en `//evil.example` | refusé — **valider après décodage** |
| `/⇥/evil.example` (TAB, LF, CR) | le caractère est retiré de l'URL → `//evil.example` | refusé — aucun caractère de contrôle |
| `/auth/login` | boucle de connexion | refusé — la cible ne peut pas être une page d'`auth` |
| valeur de 4 ko | — | refusé — longueur bornée |

Deux exigences d'implémentation qui font la différence entre un garde et une
illusion de garde :

- **Valider la valeur décodée**, celle que l'extracteur a produite — pas la
  chaîne brute de la requête. C'est le seul moment où `%2F%2F` a déjà repris
  sa forme dangereuse.
- **Valider au moment d'émettre la redirection**, dans `post_login` /
  `login_success`, et pas seulement à la capture dans `require_auth`. La valeur
  qui arrive à la page de connexion n'est jamais digne de confiance : n'importe
  qui peut composer `…/auth/login?next=…` à la main. Le contrôle à la capture
  est un confort, celui à l'émission est la sécurité.

Une seule fonction, testée par le tableau ci-dessus, appelée aux deux endroits.
Elle vit dans `auth` (`io/web/next_url.rs`) : le contrôle est purement
syntaxique, donc il part avec le BC.

**Dépendance :** l'encodage de la destination dans l'URL de connexion demande
un percent-encoder. `percent-encoding` est déjà dans `Cargo.lock` par
transitivité mais n'est **pas** une dépendance directe — l'ajouter à
`[dependencies]` (aucun téléchargement nouveau) plutôt que d'écrire un
échappement à la main.

## Ce que la redirection n'ouvre pas — le contrôle des droits

La redirection émet une **navigation GET ordinaire** vers la cible. Elle
repasse donc par toute la chaîne : `require_auth`, `space_scope_middleware`,
puis l'autorisation propre au handler. Aucune surface d'autorisation nouvelle
n'est créée — au pire, l'utilisateur est amené devant le refus qu'il aurait
obtenu en tapant l'URL lui-même.

C'est vrai **à une condition** : que la destination soit atteinte par une vraie
redirection HTTP, et jamais rendue directement côté serveur par le handler de
connexion, ce qui court-circuiterait la chaîne. Cette condition est le cœur de
la carte, et elle se prouve par un test, pas par une intention.

À noter, cela joue en notre faveur : `space_scope_middleware` répond `404` et
non `403` sur une ressource d'un autre espace — « un `403` confirmerait son
existence à qui l'énumère ». Un lien mal adressé ne révèle donc même pas que sa
cible existe.

## Conception

Le paramètre traverse le parcours de bout en bout, sans état de session :

1. `require_auth` capture la destination et redirige vers
   `/auth/login?next=<destination encodée>`.
   - Requête de navigation : la destination est `request.uri()` — chemin **et**
     chaîne de requête.
   - Requête HTMX (session expirée en cours de navigation) : c'est l'en-tête
     **`HX-Current-URL`** qu'il faut prendre, pas le chemin de la requête — ce
     dernier désigne un fragment ou un widget, qui hors de sa page rendrait un
     bout de HTML nu, sans layout.
2. `get_login` reçoit `next` en `Query`, le passe à `LoginTemplate`, qui le
   rend en champ caché du formulaire (Askama échappe le contenu par défaut).
3. `post_login` relit `next` dans le formulaire, le repasse au filtre, et
   redirige vers `/auth/login/success?next=…`.
4. `login_success` fait pointer le bouton « J'accède à l'application » sur la
   destination quand elle est valide, sur `authenticated_home` sinon.

L'écran « Bienvenue ! » est **conservé** : le sauter créerait deux parcours de
connexion à maintenir, pour un clic économisé.

Toute valeur refusée retombe silencieusement sur `authenticated_home` — un
message d'erreur sur une redirection ne dit rien d'utile à qui vient de cliquer
sur un lien, et renseignerait l'attaquant sur la règle exacte.

## Hors périmètre

L'inscription et la réinitialisation de mot de passe **ne portent pas** le
paramètre. Un lien partagé mène à une connexion ; couvrir ces deux parcours
doublerait la surface de test pour un cas rare. Si le besoin apparaît, il fera
une carte à part, qui réutilisera la même fonction de filtrage.

## Checklist

- [ ] `require_auth` capture la destination (URI pour une navigation,
      `HX-Current-URL` pour une requête HTMX) et l'encode dans l'URL de
      connexion
- [ ] La destination traverse le formulaire, le POST, puis l'écran de succès
- [ ] Fonction de filtrage unique dans `auth`, appelée à la capture **et** à
      l'émission
- [ ] Test unitaire : le tableau des valeurs ci-dessus, une assertion par ligne
- [ ] Test unitaire : une destination refusée retombe sur `authenticated_home`
- [ ] Test niveau handler (harnais de la carte 311) : login avec un `next`
      visant une ressource d'un autre espace → la réponse est le `404` du
      middleware, jamais la page
- [ ] Test e2e : lien profond → page de connexion → connexion → **la page
      demandée**, avec sa chaîne de requête intacte
- [ ] Test e2e : `?next=https://evil.example/` → accueil, et rien d'autre
- [ ] `percent-encoding` ajouté aux dépendances directes
- [ ] `make test` passe
- [ ] `make check-arch` passe — l'axe 9 doit rester muet : `auth` n'a acquis
      aucune connaissance des routes de l'hôte
