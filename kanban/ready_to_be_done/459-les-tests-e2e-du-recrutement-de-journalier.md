# Les tests E2E du recrutement de journalier

**Épic :** E15 — Recruter un journalier
**Ordre :** 5 · **Dépend de :** 458
**Conception :** `docs/specs/embaucher-un-journalier/ecran-de-recrutement/07-integration.md`

## Objectif

Prouver dans un navigateur une chaîne qui traverse **trois BCs et deux bus
d'événements**, et qu'aucun test unitaire ne voit d'un bout à l'autre.

Fichier : `tests/e2e/test_journeyman_recruitment.py`.

## Les scénarios

| Test | Ce qu'il prouve |
|---|---|
| `test_le_panneau_est_absent_sans_journalier` | le cas le plus fréquent |
| `test_un_journalier_apparait_apres_un_match` | `match_report → teams → players` |
| **`test_le_journalier_recrute_reste_dans_l_effectif`** | **le test qui compte** |
| `test_le_journalier_non_recrute_disparait` | la décision 13, bout en bout |
| `test_le_prix_se_decompose_avec_une_amelioration` | « 65 + 20 » à l'écran |
| `test_le_meme_journalier_ne_s_ajoute_pas_deux_fois` | la règle propre |
| `test_seize_dont_journaliers_autorise_le_recrutement` | le cas qui donne son sens au plafond |

## Celui qui vaut le prix de la suite

**`test_le_journalier_recrute_reste_dans_l_effectif`.**

Il traverse tout : la création à l'ouverture du rapport, le match, la
publication, le recrutement au panier, la validation de phase — et vérifie qu'il
est **toujours là** quand les autres sont partis.

C'est le seul qui prouve que l'ordre du lot d'événements tient : le basculement
en `Active` et le passage en `Dismissals` sont dans le même lot, dans cet ordre.
Si quelqu'un déplace un jour le ménage avant la validation, ce test échoue —
alors que le code compilerait parfaitement et perdrait un joueur qu'on vient de
payer.

## La non-régression, qui compte autant

**`tests/e2e/test_recruitment_phase.py` doit rester vert sans une
modification.** Ses huit cas ne concernent aucun journalier : ils mesurent donc
que le recrutement ordinaire fonctionne exactement comme avant.

Si l'un d'eux doit être adapté pour passer, c'est le signe que la carte 457 a
changé un comportement qu'elle ne devait pas toucher — le plafond, le panier ou
la trésorerie.

## Le piège de la fenêtre non câblée

Le catalogue et le panier arrivent par `hx-get` et se rechargent sur
`basketChanged`. Tout clic sur du contenu fraîchement injecté passe par
`cliquer_quand_cable` (`tests/e2e/htmx_helpers.py`).

**Pas de `sleep`.** Une durée fixe n'a aucune marge sur une machine chargée, et
c'est exactement là que la suite échouait.

## Ce que les tests ne couvrent pas

- **L'annulation d'un rapport** : elle demande de défaire un rapport en cours,
  ce que la carte 456 couvre unitairement.
- **Le garde-fou de `players`** — un recrutement sur un joueur déjà perdu : il
  demande de provoquer une course que le navigateur ne sait pas créer. Carte
  456.
- **Les quatre requêtes SQL** de la carte 454 : leurs tests d'intégration valent
  mieux qu'un parcours d'écran.

## Checklist

- [ ] Les sept scénarios
- [ ] `cliquer_quand_cable` sur le contenu injecté
- [ ] Aucun `sleep`
- [ ] `test_recruitment_phase.py` vert **sans modification**
- [ ] `make e2e`, serveur de développement lancé par l'utilisateur
