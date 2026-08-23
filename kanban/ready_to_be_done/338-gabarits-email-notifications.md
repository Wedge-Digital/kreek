# Les quatre gabarits d'email

**Spec :** `docs/specs/notifications/envoi/` (phases 4 et 7)
**État : le code est livré, la carte reste ouverte.** Il ne manque que la
vérification visuelle en client réel, qui **ne peut pas avoir lieu avant la
340** — rien n'envoie ces e-mails aujourd'hui.
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
- [ ] **Vérification visuelle à la main** : les quatre emails envoyés avec
      `EMAIL__PROVIDER=resend` sur une adresse de test, ouverts dans un vrai
      client — aucun test automatisé ne voit cela
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

## Ce qui reste, et pourquoi ça attend

La checklist demande les quatre e-mails envoyés en `EMAIL__PROVIDER=resend` et
ouverts dans un vrai client. **Aucun chemin ne les envoie avant la carte 340** :
le seul envoi existant est celui du mot de passe perdu. Les quatre rendus ont
été relus en navigateur, ce qui couvre la mise en page, les couleurs et le
logo — pas les particularités d'Outlook ni de Gmail.

- [ ] **Vérification visuelle en client réel — à faire après la 340**
