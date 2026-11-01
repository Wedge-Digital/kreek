# E20 — Coups de pouce : le panier et l'annulation

**État :** `en_cours` — 5 cartes, 626 à 630 ; 3 faites (626, 627, 629). Découpage et cadrage validés le
2026-11-01. **Cadrage :** `docs/specs/saisie-mobile/E20-cadrage.md`
Née le 2026-11-01 des maquettes
mobiles. Indépendante du mode de saisie : elle vaut sur ordinateur comme sur
mobile.

**Maquettes :** `app-mobile-real-2b-inducements-cart.html`,
`app-mobile-real-2b-inducements-cart-open.html`,
`app-mobile-inducements-done.html`, `app-mobile-inducements-cancel.html`.

## La fonction

Deux défauts du même écran.

**Le panier ne montre pas ce qu'il contient.** Le pied de page dit « N coups de
pouce sélectionnés » et ne liste que les mercenaires. Pour retirer un article
commun, spécial ou une star, il faut retrouver son onglet et décrémenter — sur
mobile, on ne le fait pas.

**Retoucher les achats du favori fausse ceux de l'outsider.** Le budget de
l'outsider dépend des dépenses du favori (`petty_cash`). Si le favori revient
modifier ses achats, ce budget change sous l'outsider sans que son panier soit
revalidé.

Décision du 2026-11-01 : plutôt que revalider, **on n'édite plus des achats
validés**. Une équipe qui a validé ses coups de pouce les voit figés ; une erreur
se corrige en **annulant globalement** les coups de pouce des deux équipes, et
l'on recommence par le favori. Le budget de l'outsider se recalcule alors de
lui-même : il n'y a rien à revalider.

Une fois les deux équipes passées, la page de choix laisse place à un
récapitulatif des achats des deux camps, qui porte l'annulation. Même route,
autre état.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 626 | Annuler les coups de pouce | événement domaine qui efface les deux paniers et les joueurs temporaires ; permis **tant qu'aucune action de match n'est saisie** (règle du 2026-11-01) |
| 627 | Des achats validés ne se modifient plus | la route `/inducements/{team_id}` d'une équipe qui a validé n'accepte plus d'écriture ; « valider » se lit dans la présence d'un `InducementsRecorded` — « Passer » en enregistre déjà un, vide |
| 628 | Le récapitulatif des coups de pouce terminés | la page de choix laisse place aux achats des deux camps, au coup d'envoi et à l'annulation confirmée dans la page |
| 629 | Le panier se déplie | le pied de page liste tous les articles ; chacun s'ajuste ou se retire sur place, par un événement sur `body` comme `removeMercenaire` |
| 630 | Les tests e2e des coups de pouce | panier, achats figés, annulation et rachat ; reprise des tests qui reviennent modifier des achats (`test_match_report_inducements`, `test_inducement_treasury`…) |

## Ce qui commande l'ordre

L'annulation et le gel des achats validés vont ensemble : geler sans annulation
rendrait une erreur de saisie irrécupérable. Le récapitulatif vient après eux,
il en est l'écran. Le panier dépliable est indépendant et peut partir le
premier.

## Conséquences sur les données de production

| Carte | Données modifiées | Effet sur l'existant |
|---|---|---|
| Annuler | un événement **ajouté**, aucun modifié | les anciens comptes rendus se rejouent tels quels ; aucun mouvement d'argent, la trésorerie n'étant débitée qu'à la publication (`teams`) ; les comptes rendus en cours sans action deviennent annulables |
| Achats figés | aucune : l'indicateur « a validé » se reconstruit au rejeu | les comptes rendus en cours dont une équipe a validé sont figés pour elle dès la livraison ; un outsider qui aurait acheté avant le favori reste tel quel |
| Récapitulatif, panier | aucune | affichage seulement ; le formulaire envoyé ne change pas |

**Perte de capacité acceptée** (décision du 2026-11-01) : aujourd'hui, un
compte rendu dépublié pour correction redevient éditable, coups de pouce
compris (`into_pre_match`). Demain, un compte rendu dépublié revient en état
« prêt à publier », et l'annulation n'est permise qu'en avant-match : **une
erreur de coups de pouce découverte après la publication ne se corrige plus.**
(Le cadrage a corrigé le motif : publier n'exige aucune action, c'est la borne
d'état qui ferme la porte.) Le récapitulatif des achats, avant le
coup d'envoi, est la dernière vérification.

## Ce que l'épic ne couvre pas

- **L'attente de l'outsider en mode « Chacun son équipe »** — le bouton « J'ai
  fini mes achats » du favori et l'état d'attente : E22.
- **L'égalité de valeur d'équipe** : carte `274-inducements-egalite-de-tv`, une
  décision de règle encore ouverte.

## Terminé quand

Sur mobile, on retire depuis le panier un article commun sans ouvrir son
onglet. Une fois les deux équipes passées, la page des coups de pouce montre
leurs achats ; « Annuler les coups de pouce » les efface tous et rouvre le choix
du favori, et l'outsider rachète avec un budget recalculé. Après la première
action de match, l'annulation n'est plus proposée.
