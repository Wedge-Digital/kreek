# Phase 8 — Les cartes : l'onglet Présences

**Entrée** : les phases 2 à 7 validées. Cette phase est la dernière de la
conception ; ce qui suit est du développement ordinaire, carte par carte.

**Épic** : `kanban/epics/to_be_refined/E16-sondage-de-presence.md`

## Les seize cartes

| N° | Carte | Vague | Dépend de |
|---|---|---|---|
| 507 | Le tirage rejoue plutôt que de laisser une équipe sur le banc | 1 | — |
| 508 | Le Calendrier passe au nouveau tirage | 1 | 507 |
| 509 | Écrire un tirage est atomique | 1 | — |
| 510 | La table des campagnes et son dépôt | 2 | — |
| 511 | `PresenceSurvey` I — ouvrir une campagne | 2 | — |
| 512 | `PresenceSurvey` II — enregistrer une réponse | 2 | 511 |
| 513 | `PresenceSurvey` III — valider un tirage | 2 | 511 |
| 514 | Les destinataires, et ceux sans adresse | 2 | 510, 511 |
| 515 | Le cycle de la campagne — ouvrir, relancer, clore, rouvrir | 3 | 510, 511, 513, 514 |
| 516 | Poser une réponse | 3 | 510, 512 |
| 517 | Tirer au sort, et l'écrire au calendrier | 3 | 507, 509, 510, 513 |
| 518 | Défaire, et réparer | 3 | 517 |
| 519 | L'onglet Présences et sa barre latérale | 4 | 510, 511 |
| 520 | Le panneau, six états | 4 | 519, 515-518 |
| 521 | Les neuf actions | 4 | 520 |
| 522 | Les tests e2e de l'onglet | 4 | 521 |

## Les quatre vagues

**1 — corriger l'existant (507, 508, 509).** Elles ne concernent pas le sondage :
l'algorithme d'appariement du Calendrier n'est ni aléatoire ni optimal, et il
laisse des équipes sans match dans 54 à 58 % des tirages en milieu de saison. Le
sondage s'appuie dessus, donc il le répare d'abord — et le Calendrier en profite
sans attendre la suite.

**2 — le socle (510 à 514).** La persistance, l'agrégat, le domain service. Rien
n'est visible à l'écran ; tout est testé unitairement.

**3 — les use cases (515 à 518).** L'orchestration. À la fin de cette vague, la
fonction marche sans interface.

**4 — l'écran (519 à 522).** La coquille, le panneau, les mutations, les tests
au navigateur.

## Trois choix de découpage, et leur raison

### 507 porte son test rouge, il n'a pas sa carte

La spec dit que le test qui manque *doit exister avant la correction, et
échouer*. Une carte dont le livrable est un test rouge ne se commite pourtant
pas : elle laisserait `make test` cassé en CI, et la règle du kanban veut une
carte compilable et testable. Le test est donc **dans** la 507, et sa checklist
impose l'ordre — écrire, voir échouer, corriger.

C'est le seul endroit du découpage où l'ordre *interne* d'une carte compte.

### L'agrégat prend trois cartes, pas une

Neuf méthodes et dix-neuf tests dépassent la session de travail que le workflow
demande. Le découpage suit les trois natures — construire, répondre, apparier —
et chacune compile et se teste seule.

L'agrégat a pourtant été **conçu d'un bloc** en phase 6, pour les trois unités :
c'est la conception qui doit être entière, pas la livraison. Découper la
conception aurait produit la troisième méthode révélant que les deux premières
avaient la mauvaise signature ; découper la livraison ne coûte rien, puisque la
forme est déjà arrêtée.

### 515 livre un expéditeur qui journalise sans envoyer

`ISurveyMailer` est déclaré par le use case ; son implémentation réelle — le
gabarit, le lien, le journal d'envoi — appartient à l'unité `reponse-coach`. La
carte livre une implémentation provisoire qui journalise, explicitement marquée.

C'est **R20 poussée à sa conclusion** : la campagne s'ouvre sans serveur de
messagerie, donc l'onglet est entièrement utilisable dès la 522 — saisie
manuelle R6, tirage, validation — avant que le premier e-mail ne parte.
L'alternative aurait été d'attendre l'unité 2 pour voir quoi que ce soit
fonctionner à l'écran, et de découvrir alors seulement ce qui cloche.

## Ce que le découpage ne fait pas

**Il ne suit pas les états de la maquette.** Une carte par état de panneau aurait
produit six cartes se disputant le même fichier de widget et le même `match`.
Les six états se livrent ensemble, en 520.

**Il ne sépare pas les gabarits de leurs handlers.** Une carte « les templates »
ne serait ni compilable seule ni testable : Askama vérifie ses champs à la
compilation contre la struct qui les porte.

**Il ne prévoit pas de carte de câblage final.** Chaque carte branche ce qu'elle
livre — le dépôt dans `main.rs` en 510, les routes en 519. Une carte de câblage
en fin de chaîne est le signe que les précédentes n'étaient pas livrables.

## Les deux corrections d'intendance faites au passage

**La carte 500 est renumérotée en 506.** Elle portait le même numéro qu'une carte
close (`done/500-le-bandeau-n-offre-que-ce-qu-on-a-le-droit-de-faire.md`) — le
cas exact que le CLAUDE.md documente. Elle est aussi inscrite dans la liste des
cartes sans épic de `kanban/epics/README.md`, où elle manquait.

**L'épic E16 est créée**, en `to_be_refined/` : ses seize cartes sont prêtes,
mais les deux autres unités de la fonction — la réponse par e-mail, l'encart du
coach connecté — n'ont pas encore de cartes. L'état suit le périmètre de l'épic,
pas celui de l'unité livrée en premier.
