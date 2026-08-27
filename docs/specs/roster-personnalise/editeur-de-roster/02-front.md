# Éditeur de roster · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-roster-editor.html`

## Ce n'est pas une page à widgets — c'est un formulaire d'un seul tenant

Le `CLAUDE.md` réserve le patron d'assemblage aux pages de trois sections
interactives **et plus**. Celle-ci en a beaucoup, mais elles éditent **un seul
objet** : un roster n'existe pas par morceaux.

Découper en widgets à endpoints séparés supposerait qu'un poste puisse être
enregistré seul, alors que sa validité dépend du roster entier — un seul
journalier, des noms de postes distincts, des limites croisées qui référencent
des postes existants. Deux widgets pourraient être valides chacun de leur côté
et faire un roster invalide.

**Un formulaire, un POST, une validation.** Tout l'état d'édition vit dans la
page jusqu'à l'enregistrement.

C'est le choix inverse de l'onglet Paramètres de compétition, qui a cinq
panneaux indépendants **parce que ses cinq réglages le sont** — modifier un
barème ne rend pas une poule invalide.

## L'état vit dans la page, en Alpine

Un roster à six postes porte **une centaine de champs** : dix pour l'identité et
les règles, treize par poste. Les tenir dans le DOM et les relire au POST serait
illisible ; les tenir dans un objet Alpine et les sérialiser en JSON à
l'enregistrement suit ce que fait déjà le magicien de compétition
(`new-competition-phase-3.html`, un `state` reconstruit à chaque rendu).

```
x-data="rosterEditor({ … état initial rendu par le serveur … })"
```

| Ce qui est front | Pourquoi |
|---|---|
| Ajouter, dupliquer, retirer un poste | l'objet n'existe pas encore côté serveur |
| Déplier l'éditeur d'un poste | état d'écran |
| Sélectionner compétences, traits, mots-clefs | recherche et filtres sur des catalogues rendus une fois |
| Le pied de cohérence | dérivé de l'état, sans aller-retour |
| Le titre qui suit le nom saisi | idem |

| Ce qui va au serveur | Quand |
|---|---|
| Les catalogues — 146 compétences, 38 mots-clefs, staff, règles spéciales | **au chargement**, une fois |
| Le roster complet | au clic sur Enregistrer, en JSON |

**Les catalogues sont rendus avec la page, jamais cherchés à la frappe.** 146
entrées font quelques dizaines de kilooctets ; une recherche serveur à chaque
touche coûterait un aller-retour pour filtrer une liste qu'on tient déjà.

## Les trois sélecteurs

Même composant, trois jeux de données. C'est la leçon des catalogues réels : une
rangée de puces ne tient pas 146 entrées.

| Sélecteur | Entrées | Filtres | Ligne secondaire |
|---|---|---|---|
| Compétences | **72** | six catégories | la description, tronquée à deux lignes |
| Traits | **74** | — | la description |
| Mots-clefs | **38** | Espèces (30) · Rôles (8) | si une Haine peut le viser |

Chacun : recherche plein texte **sur le nom et sur la description**, liste
défilante bornée en hauteur, puces retirables au-dessus.

### Compétences et traits sont deux listes, pas une

Le corpus les distingue (`type: "Trait"`) et le règlement aussi : un trait n'est
ni acheté ni gagné, il est attaché au poste. Les fondre laisserait croire qu'on
donne « Régénération » comme on donne « Blocage ».

### Quatre familles sont repliées

`HATRED_*` compte **31 entrées**, soit 42 % des traits. À plat, elles noient les
43 autres. Elles deviennent une entrée « Haine (…) » qui demande son mot-clef au
moment du choix. Même traitement pour `ANIMOSITY_*`, `LONER_*` et `BLOODLUST_*`.

**La création peut attribuer n'importe quelle Haine** (tranché en phase 1) — y
compris celles que le mot-clef ne propose pas en Haine de ligue. Le sélecteur
n'a donc aucun filtre à appliquer ici.

### Le gabarit ne doit pas être sélectionnable

Le corpus porte **à la fois** le gabarit et ses variantes : `LONER` sans nombre,
puis `LONER_3` et `LONER_4`. Le gabarit est un modèle de rédaction, pas une
compétence attribuable. Le sélecteur ne propose que la famille, et le choix du
paramètre produit la variante concrète.

**Le distinguer est mécanique** : un gabarit est une entrée dont la description
dit « le nombre indiqué entre parenthèses » — mais s'appuyer là-dessus serait
fragile. Le repérage se fait sur la liste des familles connues, tenue dans le
code : quatre familles, et une entrée du corpus qui porte le nom nu d'une
famille est le gabarit.

## Le pied de cohérence

Toujours à la même place, il change de ton — gris quand tout va, ambré quand
quelque chose manque. Le mécanisme est celui des paramètres de compétition.

Ce qu'il vérifie, entièrement côté front :

| Contrôle | Pourquoi |
|---|---|
| Au moins un poste | un roster sans poste ne s'enregistre pas |
| **Exactement un** poste journalier | c'est lui qu'on recrute en mercenaire |
| Chaque poste porte une espèce et un rôle | sans mot-clef, aucune Haine ne l'atteint |
| Les noms de postes sont distincts | ils se choisissent à l'écran de recrutement |
| Les limites croisées visent des postes existants | un poste retiré peut laisser une limite orpheline |

**Le serveur refait tout.** Le pied avertit, il n'autorise pas.

## Deux modes, un seul écran

**Un roster utilisé ne peut être ni modifié ni supprimé** (tranché en phase 1).
L'écran sert donc trois états :

| État | Ce qu'on voit |
|---|---|
| Création | tout est éditable |
| Édition d'un roster inutilisé | tout est éditable, plus un bouton « Supprimer » — le seul état où il apparaît |
| **Consultation d'un roster utilisé** | tout en lecture seule, et un bandeau qui dit **pourquoi** — « 3 équipes de cet espace jouent ce roster » |

Le troisième est le seul à maquetter en plus. Il n'est pas un formulaire
désactivé : c'est une fiche. Griser cent champs donnerait un écran illisible là
où une fiche se lit d'un coup.

**Le bandeau doit nommer la cause**, pas seulement l'interdit. Un écran qui dit
« non modifiable » sans dire pourquoi envoie chercher.

**Le même verrou porte la suppression.** Un roster utilisé ne se supprime pas :
les équipes qui le jouent en tirent leurs postes, leurs prix et leurs limites, et
le retirer les laisserait sans référentiel. Le bouton n'est donc pas désactivé,
il **n'existe pas** dans cet état — un bouton grisé invite à chercher comment
l'activer.

## Où vit l'écran

Dans l'**administration de l'espace**, à côté de la gestion des membres — c'est
là qu'on administre ce qui appartient à un espace, et le roster n'appartient
qu'à lui.

Une page de liste le précède — `assets/rawpages/html/app-roster-list.html`.

**Deux sections, deux natures** : les rosters de l'espace se *gèrent*, ceux du
règlement se *consultent*. Les mêler effacerait la seule différence qui compte
sur cet écran.

**Le compteur d'équipes ne figure que sur les rosters de l'espace**, parce qu'il
n'y décide de quelque chose que là : zéro équipe donne « Modifier » et
« Supprimer », une ou plus ne laisse que « Consulter ». Sur un roster du
règlement, non modifiable par nature, le chiffre ne changerait rien et coûterait
une lecture globale de l'event store de `teams`.

**Les actions interdites n'existent pas, elles ne sont pas grisées** — un bouton
désactivé invite à chercher comment l'activer, alors qu'il n'y a rien à activer.
Le badge dit l'état, le compteur dit la cause.

## Ce que la page ne fait pas

- **Aucun import depuis un roster existant.** Dupliquer les Elfes Sylvestres
  pour les retoucher serait le geste le plus probable d'un ligueur, et c'est
  précisément pour ça qu'il mérite sa propre décision plutôt qu'un ajout
  discret ici.
- **Aucune traduction** : le roster est saisi dans la langue de celui qui
  l'écrit. La carte 395 le reprendra si elle avance.
- **Aucun partage entre espaces.**

## Règles métier à préciser

1. **Un roster personnalisé peut-il déclarer une règle spéciale qui n'existe
   pas au corpus ?** Le sélecteur ne propose que les existantes. En inventer une
   demanderait de savoir ce qu'elle fait — donc du code, pas une saisie.

2. **Un brouillon d'équipe compte-t-il comme un usage ?** `team_drafts` porte
   ses `creation_rules` mais **pas de colonne roster** — le roster n'y est
   peut-être choisi qu'à l'étape suivante. À vérifier en phase 3 : si un
   brouillon pointe un roster, le modifier sous ses pieds le casserait.
