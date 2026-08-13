# Tests E2E — édition de l'effectif (team-detail)

**Priorité : haute**
**Dépend de :** `293-players-roster-edit-widget.md`, `294-players-roster-edit-save-endpoint.md`
**Contexte :** `players` + `teams` — tests Playwright

## Objectif

Couvrir en E2E le comportement réel du mode édition : c'est une
fonctionnalité à coordination cross-BC par événements DOM (bandeau `teams` ↔
widget `players`) — précisément le genre d'interaction qu'aucun test
unitaire ne couvre seul.

**Spec de référence :** `docs/specs/player-edition/team-detail/07-integration.md`.

---

## Scénarios

1. Renommer un joueur, enregistrer, recharger la page → le nom persiste.
2. Changer un numéro de maillot, enregistrer, recharger → persiste.
3. Vider un numéro de maillot → affiché `—` en lecture après sauvegarde.
4. Réordonner deux joueurs par glisser-déposer, enregistrer, recharger → le
   nouvel ordre persiste.
5. Saisir un numéro déjà pris par un autre joueur actif → « Enregistrer »
   désactivé, message de doublon visible, sans requête réseau.
6. Renvoyer un joueur puis attribuer son ancien numéro à un autre joueur
   actif → succès (un `Dismissed` ne bloque rien).
7. Utilisateur sans droit (ni coach, ni admin d'espace/compétition) →
   requête refusée (403).

*(Le scénario « quitter la phase pendant l'édition » a été supprimé — voir
les notes d'implémentation.)*

---

## Checklist

- [x] Fixture : équipe `Active` en état « Prête à jouer », effectif avec
      plusieurs joueurs `Active` + au moins un `Dismissed`
- [x] Scénario 1 — renommage persiste
- [x] Scénario 2 — renumérotation persiste
- [x] Scénario 3 — retrait de numéro
- [x] Scénario 4 — réordonnancement persiste
- [x] Scénario 5 — doublon bloqué front, pas de requête
- [x] Scénario 6 — numéro d'un `Dismissed` réutilisable
- [x] Scénario 7 — autorisation refusée
- [x] **Hors carte initiale** — `bypass_auth` sait connecter un membre simple
- [x] Carte ajoutée à la carte d'impact tests↔bounded-contexts (skill `test-impact`)

---

## Notes d'implémentation

**Le scénario « quitter la phase pendant l'édition » a été supprimé.** Il
décrivait un comportement qui n'existe pas : le bandeau est rendu côté serveur
au chargement, et une transition de phase déclenchée par une autre requête ne
notifie pas la page ouverte. Rien ne ferme le mode édition dans ce cas, et
aucune carte de la série ne prévoyait ce mécanisme. Le tester revenait à
inventer une garantie inexistante — l'écrire aurait fabriqué une couverture
mensongère.

**`bypass_auth` sait désormais connecter un membre simple.** `DevCoach` est
admin de l'espace E2E : sous son identité `can_spend_spp` accorde toujours le
droit, et le refus était inobservable. L'en-tête `X-Bypass-Auth-Profile: simple`
fait connecter le coach seedé sans droit d'administration. Il n'a d'effet que
si `bypass_auth` est actif — un profil de développement — et n'ouvre donc
aucune porte en production.

Le middleware ne remplace jamais une session existante : un test voulant cette
identité doit partir d'une requête sans cookie de session, ce que fait le
scénario 7 en passant par `requests` plutôt que par la page.

**L'identité simple est repérée par son nom, pas par un `legacy_id`.** Une
première version lui attribuait `legacy_id = 2` : le seed a échoué sur une base
ayant reçu les données legacy, cet identifiant appartenant déjà à un coach
importé. L'espace d'identifiants legacy n'est pas à squatter.

**Le 415 précède le 403.** L'extracteur de formulaire s'exécute avant le corps
du handler : une requête sans `Content-Type` est rejetée en 415, sans que
l'autorisation soit consultée. Le test envoie donc un corps vide mais typé,
faute de quoi il vérifierait un rejet de format en croyant vérifier un refus de
droit.

**La fixture doit recruter avant de pouvoir renvoyer.** Le domaine impose un
plancher de onze joueurs éligibles et une équipe neuve en compte exactement
onze : sans recrue, aucun renvoi n'est possible, et le scénario 6 n'aurait pas
de sujet.
