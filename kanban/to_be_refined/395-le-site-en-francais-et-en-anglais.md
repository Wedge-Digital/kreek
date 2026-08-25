# Le site en français et en anglais

**Priorité : à définir**
**Statut : à raffiner** — deviendra probablement une épic, pas une carte : le
travail se compte en centaines de fichiers, et il se découpe.

## Ce que ça touche — mesuré le 2026-08-25

| Où | Volume | Nature |
|---|---|---|
| Templates Askama | **144 fichiers, ~14 000 lignes** | le gros du texte visible |
| Libellés en Rust, couche web | ~210 lignes (`io/web` + `widgets`) | statuts, bandeaux, libellés de CTA construits côté serveur |
| Corpus de références | 11 fichiers `*_fr.json` | rosters, postes, compétences, coups de pouce, star players |
| Gabarits d'e-mail | 7, déjà sous `emails/fr_FR/` | la seule partie déjà rangée par locale |
| Tests e2e | 51 sélecteurs par texte français | casseront à la première bascule |

Aucune infrastructure existante : pas de crate i18n, aucune notion de locale,
`lang="fr"` en dur dans les deux layouts.

## La question qui décide de tout le reste

**Une instance par langue, ou une instance bilingue ?**

- **Une instance par langue.** La locale devient une variable de configuration,
  comme `REFERENCES__DIR`. Pas de sélecteur, pas de préférence utilisateur, pas
  de corpus dual, pas de locale à faire voyager par requête. Le travail se
  réduit à externaliser les chaînes — mécanique, et trois à quatre fois moins
  cher.
- **Une instance bilingue.** Deux coachs de la même ligue lisent la même page
  dans deux langues. Locale par requête, préférence par utilisateur, corpus qui
  répond dans les deux langues, e-mails choisis selon le destinataire.

Rien n'est décidé. Tout le reste de cette carte suppose la seconde.

## Architecture pressentie

**La locale voyage par requête**, jamais en variable globale : un middleware la
résout et la pose dans un `tokio::task_local!` pour la durée du handler. Un
`thread_local` serait faux — les tâches async changent de thread.

**Le point dur est Askama**, qui résout tout à la compilation : chaque struct de
template devrait porter la locale, 144 fois. L'échappatoire est un **filtre**
lisant le task-local — `{{ "team.ready"|t }}` — qui évite de toucher les
structs. **À vérifier sur un écran avant de s'engager** : Askama 0.12 résout les
filtres personnalisés dans le module du struct, donc il faudra sans doute un
`use` par fichier — une ligne, mais dans une centaine de fichiers. Pour les
messages à paramètres (pluriels, « 3 joueurs »), un catalogue **Fluent** et une
méthode explicite plutôt qu'un filtre.

## La préférence de langue

**Le navigateur est un point de départ, jamais la source de vérité** : les
e-mails de notification partent d'un **cron**, hors de toute requête. Aucun
`Accept-Language` n'y existe. Sans préférence en base, les rappels d'échéance
partent en français à tout le monde.

Cascade, le premier qui répond gagne :

1. la préférence explicite de l'utilisateur, en base ;
2. un cookie de langue, pour le visiteur non connecté — **pas la session**, dont
   le store est un `DashMapStore` en mémoire (`main.rs:583`) qui ne survit pas à
   un redémarrage ;
3. `Accept-Language`, **négocié** et non lu naïvement : tri par `q`, repli des
   variantes (`fr-BE` → `fr`), correspondance avec les langues servies —
   `fluent-langneg` + `unic-langid` ;
4. le défaut de configuration.

Stockage : une colonne sur `auth__users` (table classique, `auth` n'est pas
event-sourcé), avec un value object `Locale` à smart constructor — pas un
`String` nu. `auth` la porte parce que c'est une donnée d'identité, ce qui reste
cohérent avec son statut de BC extractible.

À l'inscription, capter l'`Accept-Language` et le figer comme préférence
initiale : le navigateur propose, l'utilisateur dispose.

## Le sélecteur

Trois publics, et l'existant n'en sert aucun : le 👤 du menu desktop
(`app-menu.html:20`) est un `<div>` inerte, les deux boutons compte du mobile
(lignes 92 et 136) pointent vers `logout()`, et **aucune route « Mon compte »
n'existe** — la maquette `app-mon-compte.html` et sa section « Préférences »
n'ont jamais été implémentées.

| Qui | Où |
|---|---|
| Visiteur non connecté | pages de connexion et d'inscription — le cas le plus oublié : un anglophone arrive sur une page en français |
| Utilisateur connecté | menu compte (desktop) et tabbar (mobile) |
| Réglage durable | page Mon compte, quand elle existera |

Cinq règles d'ergonomie :

1. **Pas de drapeaux** — un drapeau désigne un pays, pas une langue.
2. **Chaque langue écrite dans sa propre langue** : « Français », « English »,
   jamais « Anglais ». Le sélecteur doit rester lisible par qui ne comprend pas
   la langue affichée.
3. **Rester sur la même page** après le changement — `HX-Refresh: true`, un swap
   partiel laisserait le reste dans l'ancienne langue.
4. **L'attribut `lang` doit suivre** ; en dur aujourd'hui, il ferait prononcer
   l'anglais avec la phonétique française par un lecteur d'écran.
5. **Deux clics maximum**, sans quitter la page.

## Les trois endroits qui vont faire mal

1. **Le corpus de références.** Le loader lit `teams_fr.json` en dur, et le
   corpus vit hors du dépôt (`REFERENCES__DIR`). Trois voies : un dossier par
   langue, des libellés multilingues par entrée, ou un corpus unique en anglais
   sous une UI bilingue — beaucoup de ligues jouant déjà avec les noms anglais.
2. **Les 51 sélecteurs e2e par texte.** Forcer la locale en test, sinon la suite
   devient un thermomètre de traduction. Un `?lang=` **non persistant** convient
   pour ça — mais jamais de langue dans le chemin d'URL : un lien copié par un
   francophone imposerait le français à son destinataire.
3. **Les ~210 libellés en Rust**, ceux qu'on oublie parce qu'ils ne sont pas
   dans les templates. Il faudra un verrou : au minimum un test refusant une clé
   présente dans un catalogue et absente de l'autre.

## Ce qui ne se traduit jamais

Noms d'équipes, de coachs, d'espaces, articles de news, page de présentation —
tout ce que les utilisateurs saisissent. À dire dès le départ : une page
« traduite » ne le sera jamais entièrement.

## Par où commencer

Poser l'infrastructure et la vérifier sur **un seul écran autonome** : la page
de connexion. Elle a son propre layout, aucune donnée métier, et se voit avant
toute préférence utilisateur. Si le mécanisme tient là, il tiendra ailleurs.
Ensuite BC par BC, en commençant par ce que voit un visiteur.

## À trancher avant de passer en `ready_to_be_done`

- [ ] Instance par langue, ou instance bilingue
- [ ] Le sort du corpus de références
- [ ] Catalogue Fluent ou table de clés plus simple
- [ ] Filtre Askama vs champ `t` sur les structs — après vérification sur un écran
- [ ] Si le sélecteur attend la page Mon compte, ou vit d'abord dans le menu
- [ ] Découpage en épic : quelles cartes, dans quel ordre
