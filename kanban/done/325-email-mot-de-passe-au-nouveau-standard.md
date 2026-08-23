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

- [x] `lost_login.html` refondu sur la structure des maquettes
- [x] Toutes les couleurs sont des tokens de `common.css`, chacune commentée
- [x] Logo `email-logo.png`, en `{{app_url}}/static/img/…` — jamais un `data:`
      URI, que Gmail retire
- [x] `width` et `height` en **attributs HTML** sur l'image : Outlook ignore le
      CSS de dimension
- [x] Vérifier qu'aucune classe utilisée n'a perdu sa règle — le contrôle qui a
      manqué quand `.header-title` a disparu d'une maquette et laissé un texte
      sombre sur fond sombre
- [x] Le test existant de `send_reset_password_email` passe sans modification

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

## Ce qui a été fait

La structure est **copiée** d'un des quatre gabarits de la carte 338, jamais
réécrite de mémoire, et le contenu existant y est coulé. La palette est
exactement celle du site : les neuf couleurs correspondent une à une à des
tokens de `common.css`, chacune commentée de son nom. Elles restent en littéral
— un e-mail ne lit pas de feuille externe — mais on sait d'où elles viennent.

Les trois tests existants de `send_reset_password_email` passent **sans
modification**, comme la carte l'exigeait : ajouter `app_url` à la struct de
gabarit ne les touche pas.

`app_url` est construit comme `reset_url` juste à côté, à partir de
`host_domain`. Les deux partagent donc la même limite — un déploiement HTTPS les
casserait ensemble, et une seule correction les réparera. C'est délibéré :
ouvrir ici une seconde convention aurait mis deux mécanismes dans le projet au
lieu d'un à réparer.

## Quatre tests, dont celui qui avait manqué

Le contrôle des classes orphelines existait pour les quatre e-mails de
notification depuis la 338 ; ce gabarit-ci le méritait autant — c'est le seul
que les coachs reçoivent depuis toujours. Vérifié en supprimant `.header-sub` :
il tombe.

Un test interdit aussi le retour de `#6B0000`, l'ancienne couleur maîtresse
qu'aucun token ne portait. C'est tout l'objet de la carte, et rien d'autre ne
l'aurait signalé.

## Deux choses retirées au passage

Le pied portait « © 2025 BloodbowlClub » — une année en dur, **déjà fausse**.
Retirée plutôt que corrigée : une date figée dans un gabarit redevient fausse
tous les ans.

Et un `<p>` y était refermé par un `</div>`. Le genre de chose qui casse un
client e-mail strict, disparu avec la refonte.

## Choix éditorial

Le titre passe du `<h1>` du corps à l'en-tête bleu, avec « Un lien, valable
vingt-quatre heures. » en sous-titre — la forme des quatre autres. Validé à
l'écran avant commit.
