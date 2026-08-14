# L'email de mot de passe au standard visuel des emails Kreek

**Priorité : basse** — l'email fonctionne, il détonne
**Dépend de :** les maquettes d'email validées (`docs/specs/notifications/`)
**Contexte :** `assets/templates/emails/fr_FR/lost_login.html`

## Le problème

`lost_login.html` est le seul email réellement expédié aujourd'hui, et il ne
partage **rien** avec les maquettes de la fonctionnalité notifications :

| | `lost_login.html` | nouvelles maquettes |
|---|---|---|
| Couleur maîtresse | `#6B0000` rouge sombre | `#003049` bleu nuit |
| Police | Arial | Roboto / Roboto Slab |
| Palette | neuf couleurs, **aucune** dans `common.css` | tokens du site |
| Logo | le texte « Bloodbowl Club » | la marque `logo.png` |

Trois traditions visuelles coexistaient : celle-ci, celle de
`invitation-competition.html`, et la nouvelle. Les deux dernières ont été
ramenées à une seule ; celle-ci reste.

Un coach qui demande un mot de passe puis reçoit une notification de journée
reçoit deux emails qui n'ont pas l'air de venir du même produit.

## Ce qu'il faut faire

Reprendre la structure des maquettes validées — en-tête à dégradé
`#003049 → #555770`, logo `email-logo.png` en 200×81, corps blanc, carte
d'information, bouton bleu plein, pied sobre — et y couler le contenu existant :
la demande de réinitialisation, le lien, la durée de validité.

**Le contenu ne change pas.** Cette carte est une mise au même standard, pas une
réécriture du message.

- [ ] `lost_login.html` refondu sur la structure des maquettes
- [ ] Toutes les couleurs sont des tokens de `common.css`, chacune commentée
- [ ] Logo `email-logo.png`, en `{{app_url}}/static/img/…` — jamais un `data:`
      URI, que Gmail retire
- [ ] `width` et `height` en **attributs HTML** sur l'image : Outlook ignore le
      CSS de dimension
- [ ] Vérifier qu'aucune classe utilisée n'a perdu sa règle — le contrôle qui a
      manqué quand `.header-title` a disparu d'une maquette et laissé un texte
      sombre sur fond sombre
- [ ] Le test existant de `send_reset_password_email` passe sans modification

## Ce que cette carte a déjà emporté

`assets/templates/emails/en_EN/lost_login.html` est **supprimé**. Il n'était
référencé nulle part — le chemin est codé en dur en `fr_FR` dans
`send_reset_password_email.rs` — et sa structure avait divergé de la version
française sans que personne le voie.

Un dossier de traduction que rien ne lit laisse croire à un support qui n'existe
pas. La décision est prise : **le français seul**. Si l'anglais revient un jour,
il faudra d'abord une préférence de langue par coach, que ni `auth__users` ni
`spaces__user_cache` ne portent aujourd'hui.

## Pourquoi cette carte n'est pas dans le workflow

Ce n'est pas une nouvelle fonctionnalité mais une harmonisation. Elle dépend des
maquettes validées en phase 1, et se fait après elles — mais elle n'a ni spec,
ni phases, ni cartes filles.
