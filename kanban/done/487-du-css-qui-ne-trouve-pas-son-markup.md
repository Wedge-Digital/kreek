# Du CSS qui ne trouve pas son markup

**Priorité : moyenne** — trois écrans dégradés, aucun bloquant
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'utilisateur, à l'écran · **Maquette validée** avant écriture

## Trois défauts, une même famille

Aucun n'est une affaire de goût. Tous ont été constatés à l'écran puis **mesurés
dans le DOM** sur le serveur de développement.

| Où | Ce qu'on voit | Mesure |
|---|---|---|
| Onglets Classement et Classement détaillé | « Gérer les points manuels » est un lien nu | `padding: 0px`, `border: none`, fond transparent |
| Administration · Paramètres | Les cinq boutons « Enregistrer » sont énormes | 71 px de haut, 833 à 1012 px de large dans un panneau de 1060 |
| Points de classement manuels | Un champ sur quatre flotte trop haut | bas à 376 px contre 395 px pour les trois autres |

## Le bouton : six règles qui ne trouvent rien

`widgets/ranking-classement-widget.css` porte six règles pour ce bouton, toutes
scopées sous `.ranking-classement-widget`. Or le gabarit **ferme la racine ligne
74** et écrit le bouton **ligne 82**. Les règles ne rencontrent aucun markup.

C'est exactement ce que le CLAUDE.md annonce à la règle 5 des widgets — *« une
règle qui style du markup situé hors de la portée du widget mourra en
silence »* — et ce que `check-css-collisions.sh` ne peut pas voir : il vérifie
que les règles **portent** le scope, jamais qu'elles **trouvent** quelque chose.

Les deux widgets ont la faute. Le commentaire du gabarit dit que le bouton est
« dupliqué dans les deux onglets, exprès » ; la duplication a dupliqué l'erreur.

**Le correctif n'invente aucun style** : les six règles décrivent déjà le bouton
voulu — 12 px, `--main-blue`, bordure `--dark-5`, rayon 8 px, aligné à droite. Il
suffit que le bouton entre dans la racine.

### Il passe en haut, à droite

Décision prise sur la maquette. Le gabarit justifiait sa position basse : au-dessus
des onglets, le bouton s'afficherait sur Calendrier, Équipes et Statistiques. **Ce
commentaire reste** — il empêche exactement la mauvaise factorisation. Mais il
exclut « au-dessus des onglets », pas « en haut du widget ».

**Il n'y a pas de « titre du classement » auquel l'accrocher.** Les seuls titres
sont ceux des poules (`ranking-group-title`), et ils sont optionnels : une
compétition sans poule n'en affiche aucun. Le bouton va donc en tête de la racine,
sans titre ajouté — l'onglet « Classement » le porte déjà. C'est aussi le bon
niveau : les points manuels s'attribuent **par saison**, pas par poule.

## Les boutons de réglages : une classe globale mal employée

```css
.btn         { padding: var(--p2) var(--p3); }   /* 24px 36px */
.btn-primary { … width: 100%; }
```

Quatre des cinq boutons vivent en plus dans l'**en-tête** du panneau, pas en pied
de formulaire.

**La correction ne peut pas être dans `common.css`.** `.btn-primary` sert
**37 gabarits**, dont les cinq pages d'authentification où `width: 100%` est
exactement ce qu'on veut d'un bouton de connexion. Y toucher déplacerait le défaut.
La règle vit donc dans la feuille des panneaux.

## La rangée de champs : flex-end aligne les conteneurs, pas les champs

`.mp-form` pose `align-items: flex-end`. Le conteneur du motif est seul à porter
une ligne d'aide — 14 px, plus 5 px de gouttière : **les 19 px constatés,
exactement**. Et le `kreek-select` impose 49 px là où les `input` font 39 px.

Chaque champ réserve donc la place d'une aide, qu'il en porte une ou non. Le
`min-height: 14px` de `.mp-hint` existe déjà et n'attend qu'un élément à habiller.

**Le bouton « Attribuer » entre dans un champ à son tour** — sans quoi lui seul
n'aurait rien à réserver et descendrait de 19 px sous la rangée, l'inverse exact
du défaut qu'on corrige. Repéré sur la maquette, pas dans le code.

### Pourquoi pas une astuce de sélecteur

Un `::after` sur `.mp-field` réserverait 14 px partout, **y compris sous le
motif**, qui décrocherait alors dans l'autre sens. Un `:not(:has(.mp-hint))` le
ferait proprement, au prix d'un sélecteur que le prochain lecteur devra décoder.
Quatre lignes de gabarit sont plus lisibles.

## Le verrou : axe 17

### Ce qui a été écarté, et pourquoi

Le verrou évident — « une règle qui ne trouve aucun markup » — a été **prototypé
et mesuré** : sur les 42 pages du harnais visuel, **2 442 sélecteurs sur 3 588
ne rencontrent rien, soit 68 %**.

Ce chiffre ne dit pas que le dépôt porte 2 442 règles mortes. Il dit que le
harnais ne visite pas assez d'états — listes vides, formulaires en erreur, états
posés par Alpine, modales — pour que « ne trouve rien » signifie « est mort ». Un
verrou à ce taux de bruit ne serait jamais lu.

### Ce qui est retenu : deux contrôles statiques et exacts

Analyse des **gabarits**, pas du DOM : indépendante des états, des données et des
pages visitées.

**A · Une classe stylée sous une racine qui vit hors de cette racine.** Pour
chaque feuille scopée, les classes qu'elle style ; pour chaque gabarit portant la
racine, les classes qui n'en descendent **jamais**. C'est le défaut du bouton, et
il est signalé quatre fois — deux widgets, deux classes chacun.

**B · Deux attributs `class` sur un même élément.** Le navigateur ne retient que
le premier ; le second est perdu, avec toutes les règles qui le visent. Un cas
dans le dépôt, `admin/schedule.html`.

Mesuré sur 141 gabarits : **5 signalements, tous vrais, aucun faux positif.**

Le contrôle B a d'abord accusé le relevé de trésorerie, à tort : Askama rend
**une seule** branche d'un `{% match %}`, donc deux `class` séparés par un
`{% when %}` sont exclusifs, pas doublés. Le filtre est dans le code.

## Ce que la carte ne fait pas

**Elle ne touche pas à `common.css`.** Ni `.btn`, ni `.btn-primary` : 37 gabarits
en dépendent, et le défaut est dans l'emploi, pas dans la classe.

**Elle ne corrige pas les 2 442 sélecteurs sans markup.** Le chiffre mesure le
harnais, pas le dépôt. Savoir lesquels sont réellement morts demanderait une
couverture d'états qui n'existe pas ; c'est un autre sujet, et il n'est pas
urgent.

**Elle ne renomme rien.** `.schedule-actions-panel`, perdu par le double `class`,
n'est stylé nulle part : le second attribut est simplement supprimé.

## Ce que l'implémentation a appris

### Le verrou s'est retourné contre son auteur, quatre fois

**Le parseur HTML croit les commentaires.** Le commentaire que je venais
d'écrire dans le gabarit disait « le bloc vivait sous le `</div>` de fermeture ».
Le parseur y a vu une vraie fermeture, dépilé la racine, et déclaré hors scope
tout ce qui suivait : **36 faux positifs**, apparus juste après une correction
qui n'avait rien cassé. Les commentaires Askama sont retirés avant l'analyse.

**Askama ne rend qu'une branche.** Le contrôle B a d'abord accusé le relevé de
trésorerie, dont un `<tr>` porte deux `class` dans deux branches d'un
`{% match %}` — exclusives, donc jamais doublées.

**`set[str] | None` demande Python 3.10**, et le `python3` du système est en
3.9. Le script mourait sur un `TypeError` avant d'avoir rien lu. C'est le piège
de la carte 480, à un `tomllib` près.

**Un `exec` à globals et locals séparés** prive les fonctions définies de leurs
propres imports : `NameError: name 're' is not defined`. Un seul espace de noms,
comme le fait déjà `debordements.py`.

### Deux mesures qui ont changé la correction

**Le pied du panneau Général héritait du padding du formulaire** : son bouton
tombait 48 px du bord quand les quatre autres étaient à 24. Il est sorti du
`<form>` avec un `form="general-form"` — l'idiome que Poules et Visibilité
employaient déjà.

**Le bouton « Attribuer » descendait sous la rangée.** Repéré sur la maquette,
avant d'écrire une ligne : les quatre champs réservaient une place d'aide, lui
non. Sans cette remarque, la correction aurait déplacé le décrochage au lieu de
le supprimer.

### Deux tests qui ne prouvaient pas ce qu'ils annonçaient

**Une contre-épreuve mal construite.** Pour vérifier que le contrôle B ignore
les branches Askama, j'avais remplacé tout l'élément — supprimant sa racine au
passage. Le contrôle **A** a rougi, et j'ai failli lire ça comme un échec du
filtre de B. Refaite en gardant la racine, la contre-épreuve est verte.

**Le bouton d'attribution n'était pas désaligné, il était à la ligne.** Le test
le mesurait à 82 px sous les autres : `.mp-form` porte `flex-wrap: wrap`, et la
fenêtre par défaut de Playwright est trop étroite. Le test pose donc 1440 px, et
le dit — sous 768 px une media query passe même la rangée en colonne.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| axe 17 · contrôle A | une classe hors de sa racine est refusée |
| axe 17 · contrôle B | un second `class` est refusé, un `{% match %}` ne l'est pas |
| `test_bouton_points_manuels_est_style` | le bouton porte bordure et fond, e2e |
| `test_le_bouton_precede_le_tableau` | il est en tête du widget |
| `test_les_boutons_de_reglages_ne_remplissent_pas_le_panneau` | largeur au contenu, cinq panneaux |
| `test_la_rangee_d_attribution_est_alignee` | les cinq éléments ont le même bas |

Chacun falsifié en rétablissant le défaut.

## Checklist

- [x] Maquette copiée dans `assets/rawpages/html/app-css-sans-markup-avant-apres.html`
- [x] Le bouton entre dans la racine, en tête, dans les **deux** widgets
- [x] `margin-top` → `margin-bottom` sur les deux feuilles
- [x] Les cinq boutons : 128×43 px, en pied, à 24 px du bord — contre 833→1012 × 71
- [x] La rangée : cinq éléments, un seul bas mesuré à 376 px
- [x] `kreek-select` à 39 px
- [x] Le double `class` de `admin/schedule.html`
- [x] Axe 17, ses deux contrôles falsifiés, plus la contre-épreuve du filtre Askama
- [x] Six e2e (quatre paramétrés sur deux onglets), chacun falsifié
- [x] `make lint`, `make test` (1608), `make check-arch` (17 axes), `make e2e` (**347**, 0 échec)
