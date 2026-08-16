# Les quatre gabarits d'email

**Spec :** `docs/specs/notifications/envoi/` (phases 4 et 7)
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

- [ ] Les quatre gabarits, avec leurs contextes de rendu (VMs, primitives)
- [ ] `RoundEveEmail` : les deux axes en `{% if %}` / `{% match %}`
- [ ] Logo en URL absolue, dimensions en attributs HTML
- [ ] Toutes les couleurs sont des tokens de `common.css`
- [ ] Aucune classe utilisée sans règle correspondante
- [ ] Test : le HTML rendu contient l'adversaire, la journée et l'URL absolue
- [ ] **Vérification visuelle à la main** : les quatre emails envoyés avec
      `EMAIL__PROVIDER=resend` sur une adresse de test, ouverts dans un vrai
      client — aucun test automatisé ne voit cela
- [ ] `make check-arch`
