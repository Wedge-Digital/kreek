# L'angle mort de l'axe 12 : les événements que personne n'émet, ceux que personne n'écoute

**Priorité : haute — trois défauts de l'épic E15 sont passés par ce trou**
**Épic :** aucune — outillage
**Fichiers :** `scripts/check-arch.sh`, les cinq `to_app_event`,
`src/app/auth/domain/domain_event.rs`, `src/app/spaces/domain/domain_event.rs`

## Ce que l'axe 12 vérifie, et ce qu'il ne voit pas

L'axe 12 vérifie **le geste d'émission** : tout `.send(` passe par `emettre()`
ou `publier()`. C'est un bon axe, et il tient ce qu'il annonce.

Mais un événement peut être parfaitement émis dans les règles et n'aller nulle
part — ou n'être jamais émis du tout. Ces deux formes ont chacune coûté une
carte dans l'épic E15, et l'axe 12 était vert dans les deux cas.

| | La forme | Ce qu'elle a produit |
|---|---|---|
| **A** | événement **défini, jamais émis** — le bras du publisher est mort | **carte 455** : aucun journalier n'était jamais créé |
| **B** | événement **émis, sans bras** — le joker `_ => None` l'avale | **carte 457** : le coach payait son journalier et le perdait |

Les deux se ressemblent à la lecture — une chaîne événementielle qui a l'air
complète — et se distinguent par le bout qui manque.

## Ce n'est pas théorique : six fantômes aujourd'hui

Un `grep` cherchant, pour chaque variant de domain event, s'il est **construit
quelque part** en dehors de son propre fichier de définition :

| BC | Variant |
|---|---|
| `auth` | `UserPasswordResetRequested`, `UserPasswordReset` |
| `auth` | `UserEmailVerified`, `UserEmailVerificationFailed` |
| `spaces` | `SpaceArchived`, `UserInvitedInSpace` |

**Le cas d'`auth` mérite d'être nommé.** `reset_password.rs` et
`send_reset_password_email.rs` existent et fonctionnent — l'e-mail part, le
coach change son mot de passe. Mais aucun de ces deux use cases n'émet quoi que
ce soit : l'event store d'`auth` ne contient **aucune trace des
réinitialisations**. L'intention est déclarée dans l'enum, jamais câblée.

## Le code avait déjà constaté le phénomène, à la main

`spaces/domain/domain_event.rs` porte ce commentaire :

> Valait `"UserRegisteredInSpace"` — la **même chaîne** que
> `USER_SUBSCRIBED_TO_SPACE`. Deux événements distincts partageaient donc leur
> type : tout listener qui filtre dessus les attrapait tous les deux.
>
> Le défaut est resté latent parce que `UserInvitedInSpace` n'est émis nulle
> part.

C'est l'argument de la carte, écrit avant elle : **un fantôme ne masque pas
qu'un manque, il masque les défauts qu'on porte sur lui.** Deux événements ont
partagé un type d'événement persisté sans que rien ne casse, parce que l'un des
deux ne servait pas.

## Deux remèdes, de natures différentes

### Pour la forme B — le compilateur, pas un axe

Le joker `_ => None` est le seul endroit de la série où le compilateur ne
protège de rien, et son propre commentaire le dit :

> Ce joker avale silencieusement tout événement domaine qu'on oublierait de
> faire sortir du BC.

**On le supprime.** Les variants qui ne sortent pas sont nommés dans un bras
groupé — `A { .. } | B { .. } | … => None`. Un nouvel événement casse alors la
compilation, et son auteur doit trancher : il sort, ou il entre dans la liste.

C'est le mécanisme que le projet emploie déjà partout (`treasury_movement`,
`guard_active`, `est_une_amelioration`) et il vaut mieux qu'un `grep` : il ne
se contourne pas, il ne connaît pas de faux positif, et il parle au moment où
l'on écrit.

**Sa limite, à connaître** : un `match` exhaustif verrouille l'**ajout** d'un
cas, jamais le **mauvais choix** parmi les cas existants — c'est ce que la
carte 505 vient de payer. Ici cela suffit, parce que le choix est binaire et
que se tromper laisse une trace lisible dans le bras groupé.

### Pour la forme A — un axe, parce que le compilateur ne peut rien

Personne ne construit le variant, et Rust ne s'en plaint pas : les variants
d'un enum public échappent à `dead_code`. Il faut donc un axe.

**Axe 18** — tout variant d'un `*DomainEvent` est construit au moins une fois
hors de son fichier de définition et hors des tests, ou porte
`// arch:pas-emis <motif>`.

Le marqueur est obligatoire et motivé, dans le style déjà en place
(`arch:ok`, `arch:no-instrument`, `css:mort`) : une liste d'exceptions tenue
dans le script aurait dérivé, un marqueur adjacent à la déclaration ne le peut
pas.

Il couvre un cas **légitime** qu'il ne faut pas confondre avec un fantôme : un
événement dont le code émetteur a été retiré doit rester dans l'enum pour
rejouer l'historique, sans être construit nulle part.

## Ce que la carte ne fait pas

- **Elle ne câble pas `auth`.** Faire enregistrer les réinitialisations est une
  vraie décision — sur un BC extractible — et mérite sa propre carte. Ici les
  quatre variants sont marqués avec leur motif : ils deviennent visibles, ce
  qu'ils n'étaient pas.
- **Elle ne supprime aucun variant.** Aucune ligne n'a jamais été écrite sous
  ces types, donc les supprimer ne couperait aucun historique — mais cela
  effacerait aussi l'intention, et `UserPasswordReset` mérite mieux que
  disparaître.
- **Elle ne touche pas à l'axe 12**, qui n'a rien de faux : il vérifie autre
  chose.

## Ce que l'axe 18 ne verra toujours pas

Un événement émis, converti, publié — et que **personne n'écoute**. Le bout
récepteur reste hors de portée d'un `grep`, et c'est une troisième forme du
même défaut. La carte ne la traite pas ; elle la nomme pour que la prochaine
enquête ne reparte pas de zéro.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `make check-arch` sur le dépôt tel quel | l'axe 18 passe, les six fantômes marqués |
| un fantôme dont on retire le marqueur | l'axe rougit — vu, pas supposé |
| un variant ajouté sans bras dans `to_app_event` | **la compilation casse** |

## Checklist

- [x] Les cinq jokers `_ => None` remplacés par des bras groupés explicites
- [x] Axe 18 dans `scripts/check-arch.sh`
- [x] Les six fantômes marqués `arch:pas-emis` avec leur motif
- [x] `make lint && make test && make check-arch && make e2e`

`make test` : 1718 passés. `make e2e` : 368 passés, 7 sautés.

**Un échec au premier passage**, sur une course de navigation de
`test_competition_full_lifecycle` — passé isolément, puis au second passage
complet. Ce qui a permis de le classer sans y revenir n'est pas ces deux
relances mais le diff : **les seules lignes de code supprimées sont les cinq
`_ => None,`**. Chaque variant nommé dans un bras groupé y tombait déjà et
rendait déjà `None` ; le changement est neutre par construction, pas par
constat.

## Ce qui a été fait

### Le verrou du compilateur (forme B)

Les cinq jokers ont disparu, remplacés par des bras groupés nommant chaque
variant qui ne sort pas : 1 pour `competitions`, 5 pour `auth`, 4 pour
`spaces`, 25 pour `players`, 27 pour `teams`.

**Vu mordre** : un variant d'épreuve ajouté à `AuthDomainEvent` produit
`error[E0004]: non-exhaustive patterns` — le verrou parle à l'écriture, comme
prévu.

### Ce que le compilateur a trouvé tout seul, en plus

`PlayerCustomisationReverted` avait un bras — mais pour le seul
`UndoEffect::Value`. Les **trois autres formes de retrait** (`Skill`, `Stat`,
`Spp`) tombaient dans le joker.

Le commentaire voisin disait déjà qu'elles restent dans le BC, et c'était vrai.
Mais c'était **une intention écrite en français à côté d'un mécanisme qui ne la
tenait pas** : rien n'aurait signalé un cinquième `UndoEffect`. Il est
maintenant nommé dans un bras à lui, et un cinquième casse la compilation.

C'est un défaut que la carte ne cherchait pas et que le joker cachait — la
meilleure preuve de ce qu'il coûtait.

### L'axe 18 (forme A)

`scripts/arch/evenements_fantomes.py`, branché comme l'axe 17.

**Le piège qu'il fallait éviter** tient à la moitié du fichier : chercher
`Enum::Variant` dans le dépôt ne distingue pas une émission d'une réception.
Un listener qui filtre sur un événement l'écrit exactement comme celui qui le
produit — le critère naïf aurait donc déclaré « émis » tout événement seulement
consommé, c'est-à-dire un fantôme de plus. La distinction retenue est
syntaxique et sûre : **un motif de `match` est suivi de `=>` une fois son bloc
refermé, une construction ne l'est pas.**

**Le script a d'abord raté trois de ses six exceptions**, celles dont le
marqueur tient sur plusieurs lignes : il ne lisait que la ligne précédant la
déclaration. Le détail n'est pas cosmétique — un axe qui ne voit pas ses
propres exceptions rougit à tort, et un axe qui rougit à tort n'est plus lu.

**Vu mordre** : marqueur retiré de `SpaceArchived`, l'axe rougit ; remis, il
passe.

### Les six fantômes, avec leur motif

| Variant | Motif |
|---|---|
| `UserPasswordResetRequested`, `UserPasswordReset` | la réinitialisation **fonctionne**, elle n'enregistre simplement aucun fait |
| `UserEmailVerified`, `UserEmailVerificationFailed` | aucune vérification d'adresse n'existe |
| `SpaceArchived` | aucun code n'archive un espace |
| `UserInvitedInSpace` | aucune invitation n'existe |

Le premier cas est le seul qui appelle une suite : l'e-mail part, le coach
change son mot de passe, et l'event store d'`auth` n'en garde rien. C'est une
carte à ouvrir, sur un BC extractible — pas un effet de bord de celle-ci.

### Hors périmètre, corrigé au passage

`CLAUDE.md` annonçait « axes 2 à 15 » alors que le script en comptait déjà 17.
