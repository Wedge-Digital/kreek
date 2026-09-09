# Extraire la campagne de présence en BC, et laisser le tirage à `competitions`

**Priorité : à trancher — décision d'architecture, pas de code**
**Épic :** aucune — décision transverse, listée dans `kanban/epics/README.md`
**À trancher :** **après la vague 4 de E16** (cartes 519 à 522), pour la raison
donnée en fin de carte
**Mesures :** relevées le 2026-09-09, une fois la vague des use cases close
(E16 à 13/27)

## L'hypothèse retenue

**Un BC `presences` porte la campagne. Le tirage reste dans `competitions`.**

Sémantiquement, une campagne de présence n'est pas une compétition, et le
vocabulaire le dit : jetons, échéances, relances, adresses, silencieux. Rien de
tout cela n'appartient au calendrier. À l'inverse, `tirer` sert **les deux
chemins** — le Calendrier et le sondage — et les appariements vivent dans les
tables de `competitions` : le moteur reste où sont les données qu'il produit.

## Ce que les mesures disent

### Rien ne dépend du code des présences

Hors les présences elles-mêmes, seuls trois fichiers le mentionnent, et aucun ne
l'appelle :

```
context.rs            ← câblage
domain/mod.rs         ← déclaration
io/repository/mod.rs  ← déclaration
```

**C'est une feuille.** Rien ne serait cassé en la retirant, et c'est ce qui rend
l'extraction envisageable.

### Mais le code des présences n'importe pas — il **écrit**

| Ce qu'il fait | Par quoi |
|---|---|
| écrit dans `competition_pairings` | `save_pairings`, `save_pairing` |
| supprime des appariements | `delete_pairing_use_case::execute`, `clear_round` |
| émet `PairingCreated` et `PairingDeleted` | les domain events de `competitions` |
| appelle le moteur de tirage | `domain::tirage` |

Références comptées vers `competitions` : `match_day` 20, `ports` 13,
`match_day_repository_port` 10, `error` 9, `tirage` 7, puis
`entree_du_tirage`, `appariement_ecrit`, `admin::team_enrollment`,
`admin::delete_pairing_use_case`.

### La couture, fichier par fichier

**Ce qui s'extrait sans rien tordre** — aucune écriture d'appariement :

| Fichier | Ce qu'il tient de `competitions` |
|---|---|
| `presence_survey.rs` | `error`, `match_day` |
| `presence_survey_repository_port.rs` | rien |
| `presence_survey_repository.rs` | rien |
| `survey_mailer.rs` | rien |
| `survey_roster_service.rs` | `match_day`, `ports` |
| `launch` · `remind` · `close` · `reopen` · `record_answer` | `match_day`, `ports` |

**Ce qui écrit au calendrier** :

| Fichier | En plus |
|---|---|
| `draw_pairings` · `confirm_draw` · `propose_repair` · `repair_pairing` · `undo_draw` | `tirage`, `entree_du_tirage`, `appariement_ecrit`, `admin::*`, et les écritures |

## Le point dur, qu'il ne faut pas escamoter

**Les cinq use cases de tirage touchent l'agrégat de campagne** :
`peut_tirer`, `presents`, `valider_proposition`, `marquer_appariee`,
`desaccord`, `defaire_appariement`.

Couper entre « la campagne » et « le tirage » ne supprime donc pas le couplage
d'écriture : **il le retourne**. Ou `presences` écrit des appariements, ou
`competitions` mute une campagne. Le mur est le même vu de l'autre côté, et
c'est ce qu'il faudra trancher au raffinage.

### Trois façons d'en sortir, et ce qu'elles coûtent

**A — les use cases de tirage partent avec la campagne, `competitions` expose un
port impératif.** Le BC `presences` orchestre, et demande à `competitions`
d'écrire : « écris ces appariements sur cette journée », « supprime celui-ci »,
« vide cette journée ». Le moteur (`tirage`, `entree_du_tirage`,
`appariement_ecrit`) reste chez `competitions`, qui le sert aussi au Calendrier.

**Précédent existant** : `IRankingRecomputePort`, signalé dans le code comme
*« le seul port de ce BC qui **ordonne** »*. Un second port impératif est donc
une décision, pas une invention — mais c'en est une, et elle s'assume par écrit.

*C'est la piste que cette carte privilégie.*

**B — la propagation par app event.** `presences` émettrait
`PresenceDrawConfirmed { rencontres, exemptee }`, `competitions` écouterait et
écrirait. C'est le mécanisme que le `CLAUDE.md` prévoit pour propager un effet —
et il est **inadapté ici**. L'écriture deviendrait asynchrone : l'écran ne
pourrait plus annoncer « 4 matchs créés », et surtout R11 — le refus sous verrou
d'une journée déjà appariée — serait constaté dans un listener, hors d'état de
rendre son refus à l'organisateur. Un garde-fou bloquant ne se propage pas, il
se consulte.

**C — ne rien extraire.** Coût nul, bénéfice nul : rien ne dépend du code, donc
rien n'est gêné par sa place. C'est la position par défaut, et elle reste
acceptable.

## Ce que l'extraction ne doit pas défaire

**Les cartes 517 et 518 ont fusionné les deux chemins de tirage** —
`entree_du_tirage`, `appariement_ecrit` et `tirer` sont partagés entre le
Calendrier et le sondage. C'est une réparation, pas un raccourci : la carte 541 a
montré ce que coûte la divergence, R9 étant alimentée d'un côté et pas de
l'autre sans que personne le voie.

**Toute extraction qui dupliquerait le moteur est disqualifiée d'avance.** C'est
la contrainte non négociable de cette carte, et la raison pour laquelle le
tirage reste chez `competitions` plutôt que de suivre la campagne.

## Ce que le statut « BC extractible » ajouterait, ou non

`auth` et `spaces` portent ce statut, avec les contraintes du `CLAUDE.md` : ni
`AppState`, ni `AppRoutes`, ni `crate::web::`, ni `shared_kernel::bloodbowl::`,
ni `extends` vers un template hors du BC.

**`presences` ne peut pas y prétendre** : ses value objects reposent sur
`shared_kernel::bloodbowl` (`TeamId`, `MatchId`, `SeasonId`, `DateString`), et
c'est légitime — une campagne de présence n'a de sens que dans une ligue. La
question est donc un découpage **interne**, pas une préparation à l'export. À
dire dans la décision, pour que personne ne confonde les deux.

## Pourquoi trancher après la vague 4, et pas maintenant

**L'écran n'existe pas encore.** Aucune route n'est posée, et la vague 4 en
apportera onze. Le critère qui décide est précisément celui-là : l'onglet
Présences composera-t-il des widgets de **deux** BCs — la campagne d'un côté, le
tirage de l'autre — ou d'un seul ?

Si deux, l'extraction est naturelle et le pattern « page d'assemblage à widgets »
la porte déjà. Si un seul, elle ajoute une frontière que rien ne traverse.

Décider avant, c'est décider sans la moitié des faits.

## Terminé quand

La décision est écrite — extraction selon A, ou statu quo selon C — **avec son
motif**, dans ce fichier ; et, si A est retenue, les cartes de réalisation
existent avec le port impératif nommé et son coût assumé.

Ce n'est pas « le code est déplacé » : cette carte est une décision, et une
décision se close en étant prise.
