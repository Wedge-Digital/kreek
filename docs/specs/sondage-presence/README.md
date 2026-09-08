# Sondage de présence

Demander aux coachs, par e-mail, s'ils seront là pour une journée de ligue ;
suivre les réponses ; puis apparier les présents au sort, en évitant les
rencontres déjà jouées.

La fonction répond à un défaut d'organisation, pas de calcul : dans une ligue
amateur, on ne sait pas qui vient avant le jour même, et un calendrier tiré à
l'avance produit des matchs que personne ne joue.

## Décisions prises en phase 1

| Question | Décision |
|---|---|
| Réponse depuis l'e-mail | **Un lien par réponse, sans connexion.** Le coach clique « je serai là » dans sa boîte mail et c'est enregistré. |
| Grain de la réponse | **Par équipe engagée**, jamais par coach — c'est l'équipe qu'on apparie. |
| Non-répondants | **Comptés absents, listés à part.** Trois colonnes à l'écran, pas deux. |
| Emplacement du tirage | **Dans l'onglet Présences**, avec aperçu avant écriture au calendrier. |
| Écran du coach connecté | **Inclus** — un second chemin de réponse, pour quand l'e-mail se perd. |
| Défection après tirage | **Retirage partiel** : seule la rencontre touchée est refaite. |
| Ton de la page publique | **Tutoiement**, comme l'e-mail dont elle est la suite immédiate. Les pages d'auth vouvoient ; l'écart est assumé et suit d'où l'on arrive. |

## Ce que l'investigation a trouvé, et qui change le périmètre

**L'algorithme d'appariement existe, et il est faux.** `domain/match_day.rs`
expose `generate_round_pairings(teams, already_played)`, utilisé aujourd'hui par
l'onglet Calendrier. La phase 1 avait conclu qu'il suffisait de changer ce qui
alimente `teams` ; la vérification du code dit autre chose, sur les deux
objectifs à la fois.

**Il n'est pas aléatoire.** Aucun `shuffle`, aucun générateur : il énumère les
paires dans l'ordre des indices et les prend gloutonnement. Cinq appels sur la
même entrée rendent cinq fois le même appariement. Le BC sait pourtant faire —
`random_draw.rs`, le tirage des poules, utilise `StdRng::from_os_rng()` — mais
ce patron n'a jamais été appliqué aux rencontres.

**Il préfère ne pas apparier plutôt que de programmer une revanche.** Le repli
sur l'ensemble complet des paires n'intervient que si la liste des inédites est
*vide* ; tant qu'elle contient une paire, le glouton la prend et peut condamner
les équipes restantes :

```
4 équipes, déjà joué A-B, A-C, B-C  →  un seul match, A contre D
                                        B et C repartent sans jouer
```

Mesuré sur des historiques où la moitié des paires est déjà jouée — ce qu'on
atteint à mi-saison :

| Présents | Tirages laissant des équipes non appariées |
|---|---|
| 6 | 53,9 % |
| 8 | 57,8 % |
| 10 | 56,4 % |

**Et `already_played` est un `HashSet`**, donc binaire : on sait « déjà jouée »,
jamais « jouée trois fois ». Minimiser suppose de compter.

**Ce défaut est celui de l'existant**, pas du sondage : l'onglet Calendrier le
subit déjà. Les six tests unitaires de la fonction ne le voient pas — ils
portent tous sur quatre équipes à historique vide ou presque, et vérifient des
propriétés (`len`, pas de doublon) plutôt que des appariements nommés. C'est
aussi ce qui rend leur maintien possible : **introduire l'aléa n'en casse
aucun.**

Décision : **la fonction est corrigée pour tous ses appelants**, cf. R8 et R17.
Une seconde fonction réservée au sondage aurait laissé la mauvaise en service
dans le Calendrier, et deux règles d'appariement dans le même domaine.

**Le patron de jeton existe, et il n'est pas cryptographique.** `auth` fait
déjà cela pour le mot de passe oublié : un `SUlid` opaque stocké en table
(`reset_token.rs`, `insert_reset_token.sql`), dont la validité est vérifiée
dans le use case (`reset_password.rs:62`, TTL d'une heure). Aucune signature à
inventer : on copie ce patron. La formulation « jeton signé » employée pendant
la conception désignait ce mécanisme, pas une signature HMAC.

**Aucun BC hors `auth` n'a de route publique.** `main.rs:641-652` place les neuf
routeurs de BC dans `protected`, sous `require_auth` ; seul
`app::auth::router::router()` est mergé directement dans `auth_app`. Le BC
`competitions` devra donc exposer un **second routeur, public**, mergé au même
niveau qu'`auth`. C'est structurellement neuf, et cela mérite d'être décidé
consciemment en phase 3 plutôt que découvert en codant.

**Le port vers `match_report` existe déjà.** `IMatchReportStatusPort`
(`ports.rs:52`) répond « parmi ces pairings, lesquels ont un rapport publié »,
avec son adapter dans `infrastructure/competitions/`. Il sert aujourd'hui de
garde-fou à la suppression d'un pairing — exactement la nature de R13. Son
commentaire porte déjà l'argument : *consultation, pas propagation ; la
fraîcheur est critique pour une action bloquante*. **Aucun port à créer.**

**L'infrastructure d'e-mails est complète.** Quatre gabarits
(`io/email/notification_emails.rs`), un dispatcher, un service partagé
`IEmailService`, et la table `notification_deliveries` avec sa contrainte
d'unicité par destinataire. Un cinquième `NotificationType` s'y insère sans
réécrire une ligne — y compris pour la relance, qui est un second envoi sur une
clé différente.

### Le middleware CSRF n'existe pas

`CLAUDE.md`, le commentaire de `main.rs:629` et celui de `test_harness.rs:134`
décrivent tous les trois un « middleware CSRF maison qui exige `HX-Request:
true` sur toute mutation ». **`grep -ri csrf src/` ne trouve que ces
commentaires et le test qui pose l'en-tête pour lui.** Aucun fichier ne
l'implémente.

La protection contre le CSRF est donc assurée aujourd'hui **par le cookie
seul** : `SameSite::Lax` n'est pas envoyé sur une requête POST venue d'un autre
site, ce qui suffit pour les mutations. Le mécanisme fonctionne ; c'est sa
description qui est fausse, et une description fausse est ce qui fait qu'on
retire un jour la vraie garde en croyant l'autre en place.

**Cela ne bloque rien pour ce chantier** — la route publique de réponse n'a
aucune session à protéger, et n'a donc rien à contourner. **Mérite sa carte,
hors de ce chantier.**

## Découpage — trois unités, pas des pages

Le workflow procède page par page ; ici il y a un écran d'administration, un
e-mail avec sa page d'atterrissage, et un encart. Comme pour `notifications`,
les unités remplacent les pages.

| Unité | Contenu | Phase 2 |
|---|---|---|
| `onglet-presences/` | lancement, suivi, correction manuelle, tirage, validation au calendrier | oui |
| `reponse-coach/` | l'e-mail, le jeton, la route publique, la page de réponse | **sans objet** |
| `encart-competition/` | ce que voit le coach connecté sur la page de compétition | oui |

**`reponse-coach/` n'a pas de phase 2, et ce n'est pas un oubli.** Un e-mail n'a
pas d'architecture front, et la page de réponse est un rendu serveur complet :
ni HTMX, ni widget, ni événement DOM. Le bouton « finalement je ne pourrai pas »
est un lien vers le même jeton et l'autre verbe, pas un swap — la phase 3 de
l'unité a tranché la forme du lien, cf. R25. C'est inscrit dans le tableau
plutôt que laissé vide, pour la raison qui a fait trancher R5 ailleurs : une
case vide laisse croire à un trou, une case renseignée explique.

**`onglet-presences/` passe en premier** parce qu'il crée la campagne que les
deux autres lisent. L'ordre inverse obligerait à inventer une forme provisoire
puis à la refaire.

**L'agrégat se conçoit d'un bloc, pas unité par unité.** La phase 6 de
`onglet-presences/` devra donner la forme complète de `PresenceSurvey` en
tenant compte des trois unités — le workflow le dit lui-même : la troisième
méthode greffée révèle souvent que les deux premières avaient la mauvaise
signature. Concrètement, l'agrégat doit savoir répondre dès sa conception à
« un coach répond depuis un jeton » (unité 2) et « un coach connecté change
d'avis » (unité 3), pas seulement à « l'organisateur ouvre une campagne ».

Découpage écarté : **une unité par étape du cycle** (lancement / suivi /
tirage). Les trois partagent le même agrégat, le même écran et les mêmes
routes ; les séparer aurait fait trois fois les phases 3 à 7 pour un seul
panneau.

## Maquettes — phase 1

| Maquette | Écran |
|---|---|
| `assets/rawpages/html/app-competition-admin-presences.html` | l'onglet Présences, six états |
| `assets/rawpages/email/email-sondage-presence.html` | l'e-mail à deux boutons |
| `assets/rawpages/html/public-reponse-presence.html` | la page d'atterrissage du clic, quatre états |
| `assets/rawpages/html/app-competition-detail-presence.html` | l'encart du coach connecté, cinq états |

Les six états de l'onglet : aucun sondage, sondage en cours, sondage clos,
tirage proposé, journée appariée, défection après tirage.

L'onglet reprend la géométrie du **Calendrier** — journées à gauche, panneau à
droite — pour qu'on ne se réoriente pas d'un onglet à l'autre. L'e-mail reprend
les conventions des quatre déjà en place : dégradé `#003049 → #555770`, logo en
URL absolue servi en 200×81, `width`/`height` en attributs HTML pour Outlook,
tout le style en ligne. La page de réponse reprend `auth-layout`, la seule mise
en page du projet qui s'affiche sans session.

**Deux écarts au design system, assumés.** Le vert du projet (`--green`,
`#629584`) porte 3,3:1 sous du blanc et 2,9:1 en texte sur fond clair : sous le
seuil pour du 16 px, même en gras. Le bouton « Je serai là » et le libellé
« Présent » l'assombrissent en `#4A7364` et `#3F6B5C`. **Le token n'est pas
touché** — le corriger à la source dépasse ce chantier, et mérite sa propre
décision.

## Progression

| Unité | Front | Back | DTOs | Use cases | Domaine | Intégration | Cartes |
|---|---|---|---|---|---|---|---|
| onglet-presences | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| reponse-coach | — | ✅ | ✅ | ✅ | — | ✅ | ✅ |
| encart-competition | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

`—` : sans objet — la phase 2 pour un e-mail sans front, la phase 6 pour une
unité qui n'ajoute rien au domaine (l'agrégat a été conçu d'un bloc en unité 1).
Chaque cas est expliqué dans le fichier correspondant plutôt que laissé vide.

## Règles métier — tranchées en phase 1

### R1 — La réponse porte sur l'équipe engagée, jamais sur le coach

Un coach qui engage deux équipes répond deux fois, et peut venir avec l'une sans
l'autre. C'est l'équipe qu'on apparie ; une réponse par coach obligerait à
deviner laquelle joue au moment du tirage.

Conséquence sur l'e-mail : **un seul message par coach**, portant autant de
paires de boutons que d'équipes. Un e-mail par équipe multiplierait les messages
pour la même soirée, et le coach ne saurait pas lequel il a déjà traité.

### R2 — Un seul sondage vivant par journée, et jamais sur une journée de repos

La garde de repos existe déjà : `generate_pairings.rs` rend `IsRestDay` sur une
journée dont `is_rest()` est vrai. Le sondage l'applique pour la même raison —
il n'y a rien à apparier.

Un second sondage ouvert sur la même journée produirait deux jeux de jetons
vivants et deux vérités sur qui est présent.

### R3 — Un coach sans adresse connue n'empêche pas le lancement

Son équipe entre d'emblée dans la colonne « sans réponse », avec sa mention.
L'organisateur la bascule à la main s'il l'a eu au téléphone.

Refuser le lancement ferait dépendre une campagne de quatorze coachs de la fiche
incomplète d'un seul. La maquette compte déjà ces cas dans son encadré de
destinataires, à côté du nombre d'e-mails qui partiront.

### R4 — Le clic enregistre, et la page affiche aussitôt la réponse inverse

Un lien dans un e-mail peut être suivi sans que personne clique : Outlook
SafeLinks et certains antivirus visitent les URL pour les inspecter. Un `GET`
qui enregistre est donc à la portée d'une machine.

La parade n'est pas d'ajouter un second clic — ce serait renier la promesse
« un clic suffit » qui fait l'intérêt du procédé — mais de rendre le geste
réversible **sur la page même** : la réponse enregistrée s'affiche avec le
bouton opposé juste dessous. Un préfetch pose une présence que le coach voit et
corrige en un clic, et il pouvait de toute façon changer d'avis jusqu'à la
clôture.

Écarté : `GET` qui affiche, `POST` qui enregistre. Étanche, mais coûte un clic à
tout le monde pour un incident qui n'en touche qu'une fraction.

### R5 — Sans réponse vaut absent pour le tirage, mais se compte à part

Le tirage ne retient que les présences confirmées. Les silencieux ne sont pas
mélangés aux refus pour autant : ce sont les seuls que l'organisateur peut
encore convertir, par une relance ou un coup de fil. Les confondre lui ferait
perdre la seule liste sur laquelle il a prise.

### R6 — L'organisateur peut répondre à la place d'un coach, et cela se voit

Le badge « saisi par vous » distingue une réponse posée par l'organisateur d'une
réponse reçue. Ce n'est pas cosmétique : après le tirage, quand une rencontre est
contestée, la question « qui a dit qu'il venait » a deux réponses possibles et
elles n'engagent pas les mêmes personnes.

### R7 — Le jeton n'a pas de durée de vie propre

Sa validité se lit sur **l'état de la campagne** : ouverte, il répond ; close, il
affiche « le sondage est clos » ; rouverte, il répond de nouveau. Aucune date
n'est portée par le jeton lui-même.

C'est un écart délibéré avec `reset_password`, dont le jeton expire en une heure.
Là-bas le jeton *est* l'autorisation, et sa péremption est la sécurité. Ici
l'autorisation est la campagne ; donner au jeton sa propre échéance créerait un
second mécanisme d'expiration pour une seule question, et deux façons pour un
lien de cesser de marcher sans qu'on sache laquelle a joué.

**Conséquence à ne pas perdre** : rouvrir un sondage réarme les anciens liens,
sans rien réémettre.

### R8 — Le tirage poursuit trois objectifs, dans cet ordre

1. **Apparier le plus d'équipes possible.** Les coachs se sont déplacés pour
   jouer ; une soirée blanche coûte plus qu'une revanche.
2. **À nombre de matchs égal, rejouer le moins.** Et entre deux revanches
   également coûteuses, préférer la plus ancienne.
3. **À égalité, tirer au sort** (R17).

L'ordre est la règle : il dit qu'une revanche est préférable à une équipe qui
rentre chez elle sans avoir joué. C'est l'inverse de ce que fait le code
aujourd'hui, cf. l'investigation ci-dessus.

**Conséquence sur la signature** : `already_played: &HashSet<(String, String)>`
devient un **compte par paire**. Un ensemble ne permet ni le critère 2 ni son
départage — et c'est ce choix de type, pas une omission de logique, qui rendait
la minimisation impossible.

L'écran dit toujours ce que le tirage a dû concéder : le numéro de la journée où
une revanche a déjà eu lieu, comme la maquette le montre. Un tirage qui produit
une revanche sans l'expliquer passe pour un défaut ; le même avec son motif passe
pour une contrainte.

### R9 — L'exemption va à celle qui a le plus joué

Nombre impair de présents ⇒ une équipe est exemptée, et c'est **celle qui compte
le plus d'appariements** sur la saison. À égalité, R17 départage au sort.

Sans règle, le hasard peut exempter deux fois la même équipe en trois journées,
ce qui se vit comme une injustice alors que ce n'est que du hasard.

**Les appariements programmés, joués ou non.** Ils vivent dans les tables du BC,
donc le compte se lit sans interroger `match_report` ; la différence ne concerne
que les rencontres reportées, et elle ne vaut pas un port.

Le maximum se prend **sur le groupe tiré**, pas sur la saison : une poule qui
joue moins qu'une autre n'a pas à en pâtir, et un retard nul reste ainsi toujours
atteignable — ce qui garde franchissable le plancher de la recherche.

Le critère reste au **quatrième rang** des objectifs : R8.1 puis R8.2 priment.
Exempter la plus servie cède devant une revanche évitée.

#### Ce que cette règle disait avant, et pourquoi elle a changé — carte 541

Elle disait : *une équipe est exemptée, tirée parmi celles qui ne l'ont jamais
été sur la saison*. Deux défauts, l'un de mise en œuvre, l'autre de fond.

**Elle n'était pas alimentable.** Le Calendrier ne tient aucune mémoire des
exemptions passées ; `generate_pairings` passait donc un ensemble vide, le moteur
considérait toutes les équipes comme déjà exemptées, et le critère ne
départageait rien. Ce n'était pas un oubli — un commentaire l'assumait et
reportait la mémoire à l'onglet Présences — mais une règle qu'on ne peut pas
alimenter n'est pas une règle, c'est une intention.

**Elle mesurait la mauvaise chose.** Elle égalise le nombre d'exemptions ; ce qui
se vit comme une injustice, c'est le nombre de matchs. Les deux divergent dès
qu'une équipe manque des journées en se déclarant absente : elle n'a jamais été
exemptée, donc l'ancienne règle l'exemptait en priorité, alors qu'elle avait le
moins joué. Dans une campagne de présence — où l'irrégularité est le sujet même —
c'est le cas qu'il ne faut pas rater.

Le nouveau critère se lit sur les appariements déjà écrits, donc il **rend la
mémoire inutile** au lieu de la réclamer. Il n'a pas non plus de cas dégénéré :
l'équipe la plus servie existe toujours, là où « toutes les équipes ont déjà été
exemptées » demandait de relever le plancher de la recherche.

### R10 — Deux équipes d'un même coach ne se rencontrent jamais

Contrainte dure du tirage, pas une préférence : un coach ne joue pas contre
lui-même. C'est le seul cas où une paire est interdite indépendamment de
l'historique.

### R11 — Le tirage refuse une journée qui porte déjà des appariements

Même garde que `generate_pairings.rs` aujourd'hui, qui rend
`PairingsAlreadyExist`. L'organisateur vide la journée depuis le Calendrier s'il
veut retirer.

Écarté : compléter les appariements manquants. Séduisant — on saisit deux matchs
à la main puis on tire le reste — mais cela rend le résultat du tirage dépendant
d'un état que l'écran de Présences ne montre pas, et l'organisateur découvrirait
en validant que sa journée compte des matchs qu'il n'a pas tirés.

### R12 — Une modification de présence après le tirage ne refait que les rencontres touchées

L'équipe qui se décommande sort ; sa rencontre tombe ; son adversaire rejoint le
vivier des orphelins et le système propose de le réapparier. **Les autres matchs
ne bougent pas** : leurs coachs ont déjà noté leur adversaire, et refaire le
tirage entier pour une défection ferait trois mécontents pour en soulager un.

**L'exemptée est le premier remplaçant considéré**, et c'est ce qui distingue
cette règle de l'alternative « l'adversaire devient exempt » : sur le cas de la
maquette, le retirage partiel garde quatre matchs là où l'autre règle en aurait
perdu deux — l'orphelin *et* l'exemptée restant sur le banc.

Si aucun orphelin n'est disponible, l'adversaire devient exempté — et R9 ne
s'applique pas ici : c'est une exemption subie, pas tirée.

**Généralisée en phase 2** (`onglet-presences/02-front.md`, R16). Elle ne
décrivait d'abord que la défection — présent → absent. Le sens inverse existe
autant, et la règle est la même : l'arrivant tardif rejoint les orphelins, et
s'apparie avec l'exemptée s'il y en a une. Écrite sur la seule défection, elle
aurait produit un domaine sachant retirer une équipe et pas en ajouter une.

### R13 — La correction se ferme au premier rapport de match publié

Tant qu'aucune rencontre de la journée n'a de rapport **publié**, les présences
restent modifiables. Après, elles se figent : y toucher réécrirait une histoire
déjà écrite.

Le grain est « publié » et non « un score est saisi », parce que c'est ce que
`IMatchReportStatusPort` répond, et parce que c'est le bon grain : un brouillon
n'engage personne.

**Par le port, jamais par la projection locale.** `competitions` porte pourtant
`home_score` et `away_score` dans ses propres DTOs de journée — la tentation est
donc de lire ce qu'on a sous la main. Mais cette projection est alimentée par un
app event, donc en retard d'un battement, et R13 est un **garde-fou bloquant** :
la fraîcheur y est critique. C'est le critère de `CLAUDE.md` — consultation
bloquante ⇒ port synchrone, jamais cache local.

### R14 — Une équipe non appariée ne vaut rien au classement

Absente, silencieuse ou exemptée : pas de match, pas de points, aucun forfait.
Le BC `ranking` n'est pas touché par ce chantier.

Écarté : la défaite par forfait pour une absence non annoncée. Cela supposerait
de distinguer l'absence déclarée du silence, d'écrire un match sans adversaire,
et de toucher au classement — un chantier à lui seul, pour une sanction qu'une
ligue amateur applique très bien à la main.

### R15 — Le tirage refuse en dessous de deux présents, et le dit

Apparue en phase 2. `generate_round_pairings` rend une liste vide quand
`teams.len() < 2` : sans garde, l'aperçu s'afficherait vide et l'organisateur
croirait à une panne. Même motif que `skipped_group_names` dans
`generate_pairings.rs` — signaler explicitement plutôt que laisser croire à un
échec.

### R16 — Une arrivée tardive se traite comme une défection, en sens inverse

Apparue en phase 2, et absorbée par R12 ci-dessus, dont elle est le sens
manquant. Conservée comme entrée distincte parce que c'est elle qui a montré que
le tirage ne peut pas être une fonction pure `présents → appariements`.

### R17 — Le tirage est un vrai tirage, et « retirer au sort » change le résultat

Deux appels successifs sur les mêmes présents doivent donner des appariements
différents dès que plusieurs combinaisons sont également bonnes au sens de R8.
Le générateur est `StdRng::from_os_rng()`, comme `random_draw.rs`.

Sans cette règle, le bouton « Retirer au sort » de la maquette serait un bouton
qui ne fait rien — et l'organisateur qui trouve un appariement malheureux n'a
aucun recours. C'est aussi la règle qui donne son sens au mot « sort » employé
partout dans l'écran.

**Conséquence de conception, à tenir en phase 6** : l'aléa doit rester au bord.
`random_draw.rs` montre le patron — la partie déterministe (`distribute`) est
testée, le `shuffle` de `execute` ne l'est pas. Ici, choisir la meilleure
combinaison au sens de R8 est déterministe et testable ; seul le départage des
ex æquo tire au sort. Mélanger les deux rendrait la règle 1 de R8
invérifiable.


### R18 — Le tirage revérifie l'engagement au moment d'écrire

Apparue en phase 3. Une équipe peut être désinscrite de la saison entre sa
réponse et la validation du tirage. `confirm_draw` refiltre par
`filter_enrolled_team_ids`, comme `generate_pairings` le fait déjà, et signale
ce qu'il écarte — et l'aperçu filtre aussi, sans quoi il proposerait une
rencontre que la validation refuserait.

**La présence dit qu'un coach vient ; elle ne dit pas qu'il est toujours
inscrit.** Les deux faits vieillissent séparément.

### R19 — Une réponse ne vaut que pour une équipe de la campagne

Apparue en phase 4. L'agrégat refuse un `TeamId` qui n'est pas dans sa liste de
destinataires — équipe d'une autre compétition du même espace, ou jamais
engagée. `require_admin_access` vérifie que la saison appartient à la
compétition ; il ne dit rien de l'équipe.

**Distincte de R18.** R19 refuse d'enregistrer une réponse, R18 refuse
d'apparier. Une équipe désengagée après avoir répondu garde sa réponse : elle
est écartée du tirage, et l'écran le dit.

### R20 — L'ouverture de la campagne ne dépend pas de la réussite des envois

Apparue en phase 5. La campagne est persistée avant l'expédition ; un échec
d'envoi est journalisé, pas propagé. Un serveur de messagerie indisponible
bloquerait sinon une fonction qui reste utilisable sans lui — l'encart du coach
et la saisie de R6. Le journal `notification_deliveries` étant clé par
destinataire, une relance rattrape exactement ceux qui n'ont rien reçu.

### R21 — La clôture ferme le chemin du coach, pas celui de l'organisateur

Apparue en phase 5. Sur une campagne close, un jeton répond « le sondage est
clos » (R4) tandis que l'organisateur continue de poser des réponses (R6).

La règle n'était nulle part : R4, R6 et R13 la supposaient chacune de leur côté.
Sans elle, la lecture naturelle — « close veut dire close » — retirerait à
l'organisateur la seule action que la maquette lui offre sur cet écran.

### R22 — La validation revérifie la proposition, elle ne l'écrit pas sur parole

Apparue en phase 5. L'aperçu ne persistant rien, la proposition voyage par le
client et revient modifiable. `confirm_draw` et `repair` la revalident
intégralement.

Ce n'est pas de la défiance : **l'état a pu changer entre l'aperçu et la
validation** — une équipe désengagée, une réponse modifiée dans un autre onglet.
La proposition était juste quand elle a été calculée, et ne l'est plus.

### R23 — La clôture est calculée, jamais subie

Apparue en phase 6, en croisant R4 et R7 : les deux supposaient qu'une campagne
échue ne répond plus, et **aucune règle ne disait comment elle y arrive**.

Une campagne est close dès que son échéance est passée. Rien ne l'écrit :
`statut(maintenant)` croise l'échéance et la décision explicite de clôture, et
le statut n'est pas stocké.

Écarté : une tâche planifiée qui clorait les campagnes échues. La sous-commande
CLI de la spec `notifications` tourne déjà chaque jour et aurait pu s'en
charger, mais une campagne serait restée ouverte jusqu'à vingt-quatre heures
après son échéance — ses liens répondant pendant ce temps, contre ce que
l'e-mail annonce noir sur blanc.

**Conséquence : rouvrir suppose une nouvelle échéance.** Sans quoi la campagne
se referme dans la seconde. Les phases 3, 4 et 5 ont été corrigées en
conséquence — la colonne `statut` disparaît du schéma, et
`ReopenSurveyCommand` porte une `SurveyDeadline`.
### R24 — Les rencontres appartiennent à la journée, la campagne ne retient que l'exemption

Apparue en phase 7. `Appariement::Fait` portait `rencontres: Vec<PairingId>` —
une copie de ce que `competition_match_day_pairings` possède déjà. Or **quatre
chemins du Calendrier suppriment des appariements sans rien savoir d'une
campagne**, et R11 fait de l'un d'eux le chemin normal : « l'organisateur vide
la journée depuis le Calendrier s'il veut retirer ».

L'onglet aurait alors affiché « journée appariée » sur quatre rencontres
disparues, avec un bouton d'annulation qui ne défait rien. Deux vérités qu'aucune
transaction ne tient ensemble, puisque les deux écritures partent d'écrans
différents.

L'exemptée reste dans la campagne — elle n'existe nulle part ailleurs, et R9 a
besoin de l'historique des exemptions de la saison. Les rencontres se lisent sur
la journée, et « appariée » redevient un fait de la journée, juste à chaque
affichage.

**Conséquence** : `enregistrer` reçoit un `EtatJournee { figee, appariee,
rencontre_de }` là où il recevait `figee` seul — mêmes faits venus du dehors,
même agrégat qui décide. Cf. `onglet-presences/07-integration.md`.

### R25 — Le jeton désigne la réponse, pas le sens de la réponse

Apparue en phase 3 de `reponse-coach`, en tranchant la forme du lien. Un jeton
identifie **quelle équipe répond dans quelle campagne** ; ce que le coach répond
est porté par le chemin — `/presence/{token}/oui`, `/presence/{token}/non`.

La phase 1 écrivait « un lien vers l'autre jeton », ce qui aurait demandé deux
colonnes pour distinguer deux liens désignant la même réponse — sans rien
protéger, puisqu'ils arrivent dans le même e-mail.

**Conséquence assumée** : le lien reste utilisable pour changer d'avis jusqu'à la
clôture. C'est R4 — un lien à usage unique rendrait irréversible une réponse
qu'un antivirus a pu poser tout seul.

### R26 — La page publique ne dit pas si un jeton a existé

Apparue en phase 4 de `reponse-coach`. Un jeton inconnu, révoqué ou tronqué par
un client mail rendent la **même page**, et le view model de cet état ne porte
aucun champ — il n'a donc rien à divulguer même par mégarde.

La route est publique et le jeton est énumérable en principe : une page qui
distinguerait « n'a jamais existé » de « ne répond plus » ferait de la page un
oracle des jetons vivants.

Le coût est assumé : un coach dont le lien a été tronqué ne saura pas que c'est
la cause. La page le lui suggère sans rien confirmer.

### R27 — Une page qui ne prend plus de réponse dit laquelle des trois causes

Apparue en phase 5 de `reponse-coach`, en cherchant où tombe le refus de R13.

L'échéance passée, la clôture décidée et la journée déjà jouée ferment toutes le
chemin du coach — et la phase 4 les faisait tomber sur les mêmes mots. **La
troisième n'est pourtant pas une clôture** : la campagne peut être ouverte, son
échéance à venir, et la journée pourtant figée par un rapport publié, ce qui
arrive dès qu'un match se joue en avance. « Le sondage est clos » serait alors
démenti par l'échéance affichée juste en dessous.

Ne contredit pas R26 : là-bas on tait ce qu'on sait d'un jeton dont on ignore
s'il appartient à quelqu'un ; ici le porteur est légitime.

### R28 — Un coach connecté ne répond que pour ses propres équipes

Apparue en phase 2 de `encart-competition`. `Repondant` valait `Coach |
Organisateur(CoachId)` et fondait deux chemins qui n'ont pas la même
autorisation : par jeton, le lien *est* l'autorisation ; depuis l'encart, c'est
la session — et **rien ne vérifiait que l'équipe est celle du coach connecté**.
R19 contrôle que l'équipe est dans la campagne, jamais à qui elle appartient.

```rust
pub enum Repondant { Jeton, Coach(CoachId), Organisateur(CoachId) }
```

L'agrégat refuse un `Coach(id)` dont l'identifiant ne correspond pas au
`coach_id` de la réponse — **`Reponse` le porte déjà**, le domaine savait
répondre, personne ne lui posait la question. `Jeton` n'est pas contrôlé : lui
faire porter un `CoachId` comparerait la réponse à elle-même.

**Le canal ne se persiste pas.** `saisi_par_admin` garde ses deux cas, et un
`NULL` relu rend `Coach(coach_id de la réponse)`. La question qu'on se pose
après coup — « qui a dit qu'il venait » — a deux réponses possibles, pas trois.

**Ce que ça dit du pari « l'agrégat se conçoit d'un bloc »** : il le passe à
moitié. La forme était bonne, les trois unités appellent bien le même
`enregistrer` — mais deux chemins avaient été fondus en une variante, et seul le
troisième appelant l'a fait voir.

### R29 — L'encart dit combien, jamais qui

Apparue en phase 4 de `encart-competition`. Le sous-titre annonce « 9 équipes
ont déjà confirmé » ; il n'annoncera jamais lesquelles, ni qui a décliné.

Une réponse est donnée à l'organisateur, qui apparie — elle n'est pas publiée
aux autres coachs. Afficher la liste ferait de l'encart un tableau de présence
collectif, avec la pression sur celui qui n'a pas répondu, et la possibilité de
choisir sa soirée selon les adversaires présents — ce que le tirage au sort
existe précisément pour empêcher.

Vaut pour l'encart et la page publique. L'onglet d'administration montre les
noms : c'est son objet.

### R30 — Un désistement après tirage le dit, et ne promet rien

Apparue en phase 5 de `encart-competition`, en croisant R12 et R16 avec le fait
que l'encart existe : les deux règles décrivaient la défection **vue de
l'organisateur**, seul écran qui existait alors.

Quand la réponse d'un coach défait une rencontre déjà tirée, l'encart le lui dit
et s'arrête là. **Il n'annonce pas de nouvel adversaire** — la réparation est une
proposition que l'organisateur valide, et annoncer un remplacement non confirmé
produirait deux coachs qui se croient appariés et un match qui n'existe pas.

**Il ne se tait pas non plus** : un désistement silencieux laisserait le coach
croire qu'il a changé une case, alors qu'il vient de défaire un match que son
adversaire avait noté.

## Ce que ces règles impliquent pour les phases suivantes

- **R7 décide la forme du jeton, donc de la table.** Sans échéance propre, la
  ligne n'a pas de colonne d'expiration et la requête de validation joint la
  campagne. C'est un choix de schéma qui découle d'une règle, et il tombe en
  phase 7 autant qu'en phase 6.
- **R12 est la règle qui commande le domaine.** Elle interdit de modéliser le
  tirage comme une fonction pure `présents → appariements` : il faut pouvoir
  refaire *une* rencontre en connaissant les autres. C'est la contrainte qui doit
  guider la signature des méthodes de l'agrégat, et elle n'apparaît qu'au sixième
  état de la maquette.
- **R4 fait de la route publique le point d'architecture le plus neuf**, cf.
  l'investigation ci-dessus : `competitions` n'a jamais exposé de route hors
  `require_auth`.
- **R8, R9, R10 et R17 se composent, et leur ordre est désormais écrit.** Il se
  lit ainsi, du plus contraignant au plus souple :

  | Rang | Règle | Nature |
  |---|---|---|
  | 1 | R10 — jamais deux équipes d'un même coach | contrainte dure, jamais violable |
  | 2 | R8.1 — apparier le plus d'équipes possible | objectif |
  | 3 | R8.2 — rejouer le moins, et le plus anciennement | objectif |
  | 4 | R9 — exempter une équipe qui ne l'a jamais été | préférence, si effectif impair |
  | 5 | R17 — départager au sort | arbitrage final |

  Seul le rang 1 refuse ; les autres cèdent dans l'ordre. L'écran dit ce qui a
  été concédé au rang 3, comme la maquette le montre déjà.

- **R8 et R17 corrigent l'existant, et débordent donc du chantier.**
  `generate_round_pairings` sert aujourd'hui l'onglet Calendrier, qui souffre du
  même défaut sur plus de la moitié des tirages. La correction lui profite, mais
  elle change le comportement d'une fonction en service : elle mérite sa carte,
  ses tests de non-régression, et d'être livrée avant ce qui s'appuie dessus.
  Ses six tests actuels survivent tels quels — ils portent sur des propriétés,
  pas sur des appariements nommés — mais ils ne couvraient pas le cas qui casse,
  et la carte devra l'ajouter.
- **R6 et R13 se lisent ensemble** : la traçabilité de qui a répondu n'a de sens
  que tant que la correction est ouverte. Après le figeage, elle devient un
  historique.

- **R5 et R6 se modélisent ensemble, en un seul type.** « Sans réponse » est un
  troisième état, pas un booléen absent ; et l'horodatage comme l'auteur n'ont
  de sens que là où une réponse existe. La forme retenue est un enum à données
  portées (`onglet-presences/03-back.md`), et elle vaut pour les **trois
  unités** : c'est le même `record_answer` qu'appellent l'organisateur, le jeton
  et l'encart. Un booléen nullable aurait rendu exprimable une réponse sans
  auteur, et laissé à chaque appelant le soin de ne pas le faire.
