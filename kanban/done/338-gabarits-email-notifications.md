# Les quatre gabarits d'email

**Spec :** `docs/specs/notifications/envoi/` (phases 4 et 7)
**État : terminée le 2026-08-25.** Le code était livré depuis la phase 4 ; la
carte attendait la vérification en client de messagerie réel.
**Dépend de :** 337 *(pour `RoundParticipation`)*
**Ouvre :** 339

## Objectif

Convertir les quatre maquettes validées en gabarits Askama.

| Gabarit | Maquette |
|---|---|
| `competition_registration_open.html` | `invitation-competition.html` |
| `competition_round_eve.html` | `email-journee-demain.html` |
| `competition_round_closing.html` | `email-fin-de-journee.html` |
| `competition_registration_deadline.html` | `email-date-limite-inscription.html` |

Destination : `assets/templates/emails/fr_FR/`, où vit déjà `lost_login.html`.

## Conception

### Deux axes de variation dans la veille de journée, pas un

| Axe | Ce qui change | Piloté par |
|---|---|---|
| type de journée | la ligne « clôture » apparaît ou non | `date_end: Option<String>` |
| coach avec ou sans match | le bloc des matchs, ou « tu ne joues pas » | `participation` |

**Quatre combinaisons, toutes atteignables** — une journée à date fixe pour un
coach qui ne joue pas est ordinaire. Les confondre en une seule condition
produirait un email amputé pour un quart des cas.

`participation` est un **enum**, pas un `Vec` : un `Vec` vide se rendrait en
silence et la ligne « tu ne joues pas » disparaîtrait sans que rien ne proteste.

### Contraintes d'email, pas de page web

- Logo en `{{app_url}}/static/img/email-logo.png` — **jamais** un `data:` URI,
  que Gmail retire.
- `width` et `height` en **attributs HTML** : Outlook ignore le CSS de dimension.
- Aucune dépendance à une feuille de style externe.
- `app_url` porte son schéma, depuis la configuration — ne pas recopier le
  `http://` en dur de `send_reset_password_email`.

### Le contrôle qui a manqué une fois

Vérifier qu'aucune classe utilisée n'a perdu sa règle. C'est ce qui a laissé un
texte sombre sur fond sombre pendant la phase 1, quand une substitution a mangé
`.header-title` et `.header-sub`.

## Checklist

- [x] Les quatre gabarits, avec leurs contextes de rendu (VMs, primitives)
- [x] `RoundEveEmail` : les deux axes en `{% if %}` / `{% match %}`
- [x] Logo en URL absolue, dimensions en attributs HTML
- [x] Toutes les couleurs sont des tokens de `common.css`
- [x] Aucune classe utilisée sans règle correspondante
- [x] Test : le HTML rendu contient l'adversaire, la journée et l'URL absolue
- [x] **Rendu par le vrai chemin d'envoi**, pas par un test : une compétition
      d'essai créée par le parcours réel, les dates ajustées pour que les trois
      notifications du cron tombent dues, puis `send-notifications` avec
      `EMAIL__PROVIDER=console`
- [x] `make check-arch`

## Ce qui a été fait

Les quatre maquettes copiées puis converties. Les variantes vivaient en
**commentaires HTML** ; elles deviennent des `{% if %}` et `{% match %}`. Les
structs de rendu vont dans `src/app/competitions/io/email/` — rendre un e-mail
est de l'IO, et la 339 n'aura qu'à les remplir.

### Deux écarts avec la maquette

Le pied de page disait « parce que ton équipe **X** participe ». Un coach peut
en aligner deux, et la clé d'idempotence ne portant pas d'équipe il ne reçoit
qu'un e-mail : le singulier serait faux une fois sur deux. Il dit désormais
« parce que tu participes à ». Le titre du bloc de matchs passe au pluriel au
même titre.

### Le contrôle « à la main » est devenu un test

La carte demandait de vérifier de visu qu'aucune classe n'a perdu sa règle, en
rappelant qu'une substitution avait mangé `.header-title` et `.header-sub` en
phase 1 — texte sombre sur fond sombre. **À la main, ce contrôle ne se refait
pas.** Il est écrit en test, et vérifié en supprimant cette règle-là : il rend

```
gabarit 0 : classes sans règle — ["header-title"]
```

## La vérification finale, et ce qu'elle a trouvé

Une compétition d'essai a été créée dans la base de développement **par le
parcours réel** — le même que celui de la suite e2e —, ses dates ajustées pour
que les trois notifications du cron tombent dues le jour même, puis
`send-notifications` lancé avec `EMAIL__PROVIDER=console`.

Ce chemin-là compte : ce n'est pas un test qui rend un gabarit avec des données
inventées, c'est la commande de production qui résout ses destinataires,
compose ses sujets et rend ses corps.

| Vérifié | |
|---|---|
| `due=3` | exactement les trois du cron, ni plus ni moins |
| Relance | aucun renvoi — **l'idempotence tient** |
| Date limite sans invités | ne vise personne, ce qui est le comportement correct |
| URL | absolues une fois `HOST_DOMAIN` renseigné |

**Le rendu a immédiatement montré un défaut de configuration** que ni les tests
ni une relecture en navigateur n'avaient vu :

```html
<img src="http:///static/img/email-logo.png">
```

Trois barres obliques. `AppConfig::app_url()` fabrique `format!("http://{d}")`
sans vérifier que `d` est vide, et rend `"http://"` — une URL syntaxiquement
valide, sans hôte, dont le logo ne s'affiche jamais et dont les liens ne mènent
nulle part. `.env.dev` et `.env.remote.demo` ont tous deux `HOST_DOMAIN=` vide.

C'est **exactement ce que cette case attendait** : un défaut que seul le rendu
réel donne à voir.

**L'ouverture en client de messagerie n'a pas eu lieu.** Elle demandait un
envoi réel depuis une clé Resend vers une adresse de test ; l'utilisateur a
jugé le sujet maîtrisé et la carte close sans elle. C'est écrit ici plutôt que
coché, pour que personne ne croie plus tard qu'Outlook et Gmail ont été
regardés.

### Ce qui n'est pas un défaut, malgré les apparences

La production configure `HOST_DOMAIN=bloodbowlclub.com`, **sans schéma**, donc
`app_url()` complète en `http://`. Mesuré plutôt que supposé :

```
http://bloodbowlclub.com/  →  301  →  https://bloodbowlclub.com/
```

Le serveur redirige : les liens des e-mails de production fonctionnent.
