# Les comptes d'expédition ne vont nulle part

**Priorité : moyenne — une fonction qui marche et ne le dit pas**
**Épic :** E16 — Sondage de présence
**Dépend de :** 527 (qui produit les trois compteurs)
**Fichiers :** `src/app/competitions/io/web/admin/presences_actions.rs`,
`.../presences_widgets.rs`, un gabarit de bandeau

## Ce qu'on a découvert

`LaunchOutcome` porte `destinataires`, `sans_adresse`, `envoyes`,
`deja_envoyes` et `echecs`. `RemindOutcome` porte `relances`, `envoyes`,
`deja_envoyes` et `echecs`. **Aucun n'atteint l'écran** :

```rust
match issue {
    Ok(_) => succes(),          // presences_actions.rs — les compteurs tombent ici
```

`succes()` rend un corps vide avec `HX-Trigger: presenceChanged`, les deux
widgets se rechargent, et le panneau relit la campagne — qui ne stocke aucun de
ces nombres. Ils sont calculés, journalisés, puis perdus.

## Pourquoi ça compte

R20 dit qu'un échec d'envoi est **journalisé, pas propagé**, et la carte 515
écrivait que `echecs` « est un fait que l'écran annonce ». L'écran n'annonce
rien. Concrètement : un organisateur dont le serveur de messagerie est en panne
lance sa campagne, voit le panneau se recharger normalement, et **croit ses
quatorze e-mails partis**. Il l'apprendra par un coach, ou pas du tout.

`deja_envoyes` est le cas le plus parlant : il ne peut se produire que sur un
double clic ou une relance du même jour, c'est-à-dire exactement quand
l'organisateur se demande si son action a été prise en compte.

Le compte `sans_adresse` (R3), lui, **est** affiché — mais avant le lancement,
dans le panneau de préparation. Après, plus rien.

## Piste, à discuter

Un bandeau rendu par la réponse du POST, plutôt qu'un corps vide : « 12 e-mails
envoyés · 2 coachs sans adresse · 1 échec ». Il faut alors décider s'il remplace
`succes()` pour les dix actions ou seulement pour les deux qui expédient, et
comment il cohabite avec `HX-Trigger: presenceChanged` — le fragment de succès
ne vise aucune cible aujourd'hui.

## Les trois questions, tranchées le 2026-09-15

**Ni toast, ni persistance : une ligne dans le panneau, relue du journal.**

*Un toast ?* Non — et la question s'est réglée d'elle-même en cherchant sa
réponse : **le projet n'a pas de toast.** Trois handlers de `team_creation`
émettent `HX-Trigger: {"showToast": …}` et personne n'écoute. Ni composant, ni
écouteur, ni feuille. Poser un toast n'était donc pas l'option légère qu'on
croyait, mais un composant à construire — c'est l'objet de la carte **547**.

*Que montre-t-on quand tout va bien ?* La ligne, toujours. « Est-ce parti ? » se
pose aussi quand tout va bien, et une ligne qui n'apparaîtrait qu'en cas d'échec
laisserait l'organisateur sans réponse le reste du temps.

*Faut-il persister ?* **Non, et il n'y a rien à ajouter au schéma.** La vérité est
déjà stockée : `competition_notification_deliveries` porte une ligne par coach
réservé, `sent_at` renseigné à la confirmation. `count(*)` donne les créneaux,
`count(sent_at)` les envois attestés, la différence les échecs. Un compteur écrit
sur la campagne serait une **seconde vérité** à tenir d'accord avec celle-là, pour
économiser une requête sur un panneau qu'on ouvre à la main.

Corollaire : la réponse survit au rechargement sans qu'on ait rien fait pour, et
c'est ce qui compte — la question se repose le lendemain.

## Le piège que la carte ne voyait pas

**Le journal compte des coachs, le roster compte des équipes.** R1 envoie un
message par coach ; `DestinatairesVm` compte `equipes` et `sans_adresse` en
équipes. Écrire « 12 envoyés sur 14 » en mêlant les deux aurait été faux dès qu'un
coach engage deux équipes — et juste partout ailleurs, donc invisible.

La ligne dit donc des coachs, et le compte des équipes sans adresse reste **à
part**. C'est le piège de la carte 495 déplacé d'un cran, et le test
`la_ligne_compte_des_coachs_et_non_des_equipes` est là pour ça.

## Ce qui a été livré

- `count_deliveries_by_round.sql` — une requête groupée par type et par date
  d'envoi. Le groupement par date est ce qui sépare l'ouverture des relances :
  l'ouverture porte l'échéance, chaque relance porte son jour.
- `NotificationDeliveryRepository::count_by_round` et son `DeliveryCountDto`, qui
  rend `reserves` et `attestes` bruts plutôt qu'un champ `echecs` — la
  soustraction se lit aussi bien, et ne fige rien.
- `ExpeditionVm` / `EnvoiVm`, dans le panneau « en cours » **seulement** : c'est
  là que l'organisateur regarde les réponses arriver, donc là qu'il se demande si
  ses e-mails sont partis. Après le tirage la question ne se pose plus.
- Le dépôt rejoint `CompetitionsContext`, à côté des autres.
- `succes()` **n'a pas changé** : le panneau se recharge déjà sur
  `presenceChanged` et relit le journal. Les dix actions sont intactes.

Deux lignes à l'écran :

```
Envoi initial            4 coachs joints sur 4
Dernière relance du …    3 coachs joints sur 4      1 en échec
```

## Un détail de langue, corrigé après l'avoir vu rendu

Le premier jet écrivait « 4 coach(s) joint(s) » — la forme qu'on emploie quand on
renonce à accorder. Le reste du projet accorde (« 1 équipe a déjà confirmé »
contre « 9 équipes ont déjà confirmé »), et l'accord se fait dans le gabarit, sur
les nombres que le VM rend bruts. Vu en lisant le fragment réellement servi, pas
en relisant le code.

## Checklist

- [x] La requête et son DTO, groupés par type et par date
- [x] `ExpeditionVm`, qui compte des coachs et tient les équipes à part
- [x] La ligne dans le panneau « en cours », et sa feuille
- [x] Cinq tests d'intégration sur le dépôt, cinq tests unitaires sur le VM
- [x] Un scénario e2e : le panneau relu par un `GET` neuf porte la ligne
- [x] `make lint`, `make check-arch`, `make test`, `make e2e`
