# La campagne de présence reste dans `competitions`

*Titrée d'après la décision et non d'après l'hypothèse : le fichier vit dans*
*`done/`, et un titre qui annonce une extraction qui n'aura pas lieu tromperait*
*qui parcourt le dossier. L'hypothèse examinée est en tête de la section*
*suivante.*

**Priorité : à trancher — décision d'architecture, pas de code**
**Épic :** aucune — décision transverse, listée dans `kanban/epics/README.md`
**Tranchée le 2026-09-11 : statu quo (option C).** La décision et son motif sont
en fin de carte ; ce qu'elle garde est vérifié par l'axe 18 de `check-arch`.
**Mesures :** relevées le 2026-09-09, une fois la vague des use cases close
(E16 à 13/27)

## L'hypothèse qui était sur la table

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

## LA DÉCISION — 2026-09-11 : **C, statu quo**

Prise une fois l'écran livré (cartes 519 à 522) et l'unité 2 close (523 à 528),
c'est-à-dire au moment que cette carte s'était fixé. Le tirage reste dans
`competitions`, et la campagne aussi.

### Le critère que la carte s'était donné a répondu : **un seul BC**

`etat_du_panneau` (`presences_widgets.rs:251`) choisit entre cinq états en lisant
**les deux côtés à la fois** : `survey.desaccord(journee)`, puis
`journee.appariee()`, puis `survey.statut(aujourd_hui)`. L'aperçu du tirage et la
proposition de réparation ne sont pas des widgets voisins — ce sont des **états du
même panneau**.

Et la lecture croisée n'est pas un raccourci : c'est R24, écrite après un défaut
réel. La campagne se souvient d'avoir été appariée, quatre chemins du Calendrier
suppriment des appariements sans rien savoir d'une campagne, et le panneau
annonçait « Journée appariée » sur une journée vide. La correction a été de lire
la journée, jamais la mémoire de la campagne.

**Une frontière passerait donc au milieu de cette fonction.** Sous l'option A,
chaque rendu du panneau demanderait un appel de port pour savoir si la journée est
appariée, et les quatre chemins du Calendrier devraient franchir la frontière en
sens inverse pour remettre la campagne d'accord. Le mur est le même vu de l'autre
côté, comme cette carte l'annonçait — l'écran a seulement montré où il passe.

### Trois coupures nouvelles, nées après la mesure

Les cartes 526 et 527 ont branché les présences sur la plomberie de notification :

| Partagé | Avec |
|---|---|
| `notification_emails.rs` | six gabarits dans un fichier, dont les deux du sondage |
| `NotificationType` | deux variantes dans l'enum des quatre du cron, et le test de figeage tient les six |
| `competition_notification_deliveries` | `survey_mailer.rs` y écrit via `NotificationDeliveryRepository` |

La troisième tranche seule. Extraire, c'est **écrire du SQL dans une table de
`competitions`** — l'exacte violation que la souveraineté des données nomme — ou
donner aux présences leur propre journal, **avec sa copie de `claim`/`confirm`**.
Or cette carte disqualifiait d'avance toute extraction qui dupliquerait un
mécanisme partagé, et pour le motif que la 541 a établi : la divergence ne se voit
pas.

### Le compte des références a monté, pas baissé

| | 2026-09-09 | 2026-09-11 |
|---|---|---|
| `match_day` | 20 | **27** |
| `ports` | 13 | **15** |
| `error` | 9 | **10** |
| `tirage` | 7 | **9** |
| `use_cases::admin` · `entree_du_tirage` · `appariement_ecrit` | — | **6 · 4 · 2** |
| `io::web` · `notification_delivery` · `io::email` | — | **6 · 1 · 1** |

Cinq cartes de travail ont **épaissi** la couture. C'est une mesure de direction :
les deux poussent ensemble.

### L'argument contraire, qui reste vrai

`competitions` fait **39 414 lignes de Rust**, presque le double de `teams` qui
suit ; le périmètre des présences en fait **11 636**, soit 30 %.

| Part | Lignes |
|---|---|
| la campagne — agrégat, dépôt, expédition, use cases, page publique | 5 940 |
| les cinq use cases de tirage | 2 184 |
| l'onglet — coquille, widgets, dix actions | 2 541 |
| doubles et tests de dépôt | 954 |

C'est un argument de **navigabilité**, pas de couplage, et l'extraction ne
réduirait pas le total : elle le déplacerait en ajoutant une frontière et un port
impératif. Ce que la taille réclame vraiment — s'orienter dans 39 000 lignes —
`use_cases/presences/` le donne déjà.

**Ce n'est pas un argument écarté, c'est un argument perdant.** S'il redevenait
majoritaire, ce serait un fait nouveau, et cette carte se rouvrirait.

### Le statut « BC extractible » n'est toujours pas en jeu

Inchangé depuis la rédaction : les value objects reposent sur
`shared_kernel::bloodbowl`, ce qui est légitime — une campagne de présence n'a de
sens que dans une ligue. La question était un découpage **interne**, jamais une
préparation à l'export.

## Ce que la décision garde : l'axe 18

Le périmètre **est** une feuille : hors `context.rs`, `router.rs`,
`admin_page.rs` et les `mod.rs`, rien dans le projet n'appelle le code des
présences. Cette propriété était vraie **par accident**, et rien ne la vérifiait.

`scripts/check-arch.sh` axe 18, bloquant, la rend tenue. Il ne remplace pas une
frontière — il n'y en a pas. Il empêche qu'on en perde la possibilité sans s'en
apercevoir.

Le câblage y est **nommé, pas marqué** : pas de `// arch:ok`, qui s'essaimerait au
fil des ajouts, chacun justifié sur le moment. Ajouter un point d'entrée demande
une ligne dans le script, qui se relit en revue.
`src/web/tests/test_route_publique_presence.rs` y figure parce qu'il **est** un
garde-fou — il demande la route publique sans cookie contre le routeur de
production — et le lui interdire supprimerait une garde pour en satisfaire une
autre.

L'axe a été éprouvé dans les deux sens : vert sur le code actuel, rouge sur un
import posé exprès depuis `teams`.

## On rouvrirait si

- le tirage cessait d'être partagé entre le Calendrier et le sondage ;
- le journal d'envois devenait un service transverse, hors de `competitions` ;
- le panneau se scindait en deux fragments dont chacun ne lit qu'un côté.

Aucune de ces trois n'est à l'ordre du jour. Les deux premières seraient des
chantiers en soi ; la troisième contredirait R24.

## Terminé quand

La décision est écrite — extraction selon A, ou statu quo selon C — **avec son
motif**, dans ce fichier ; et, si A est retenue, les cartes de réalisation
existent avec le port impératif nommé et son coût assumé.

Ce n'est pas « le code est déplacé » : cette carte est une décision, et une
décision se close en étant prise.

**C'est fait.** C a été retenue, avec son motif ci-dessus. Aucune carte de
réalisation n'est donc à écrire — l'option A n'a pas été retenue — et le seul
livrable de code est l'axe 18, qui tient la propriété sur laquelle la décision
s'appuie.
