# Le site en français et en anglais

**Priorité : à définir**
**Statut : à raffiner** — deviendra probablement une épic, pas une carte : le
travail se compte en centaines de fichiers, et il se découpe.

## Ce que ça touche — mesuré le 2026-08-25, recompté le 2026-08-26

| Où | Volume | Nature |
|---|---|---|
| Templates Askama | **144 fichiers, ~14 000 lignes** | le gros du texte visible |
| Libellés en Rust | **869 littéraux** — `io/web` 268, `use_cases` 182, **`domain` 120** | statuts, bandeaux, CTA, **et les messages d'erreur du domaine** |
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

**Tranché le 2026-08-26 : instance bilingue**, et ce sont les e-mails qui
l'imposent. Les relances partent d'un **cron**, hors de toute requête HTTP :
aucun `Accept-Language` n'y existe. Sans préférence linguistique en base, tous
les rappels partent dans une seule langue **quel que soit le déploiement** — une
instance par langue ne résout donc pas le problème complet. Et dès lors que la
colonne existe, le surcoût du bilingue par requête est marginal.

## Ce que l'écosystème Askama offre — rien

**Askama 0.12.1 n'a aucun support i18n.** Ni filtre `localize`, ni feature, ni
trace de Fluent dans la crate ni dans `askama_derive`. Le support qui existait
en 0.10/0.11, adossé à `fluent-templates`, a été retiré. La bonne pratique de
l'écosystème se résume à : on apporte son catalogue et son filtre.

**Et la génération de code contraint la solution.**
`askama_derive-0.12.5/src/generator.rs:1282` compile un filtre non intégré en :

```rust
filters::t(...)?          // chemin NON qualifié, et un ? à la fin
```

Deux conséquences fermes :

- le filtre doit rendre un `Result` ;
- `filters::` se résout **dans le module où vit la struct de template**, donc il
  faut un `use crate::i18n::filters;` dans chaque fichier qui déclare un
  `#[derive(Template)]` — **96 fichiers** (112 structs), une ligne chacun. Pas
  144 comme la première version de cette carte le laissait croire.

L'alternative `{{ self.t("clef") }}` fonctionne — `app-menu.html:26` appelle
déjà `self.competitions_active()` — mais une méthode de trait exige aussi son
`use`. Même coût, aucun avantage.

Un filtre ne voyant pas la struct, **la langue doit être ambiante** :
`tokio::task_local!`, jamais un `thread_local`.

## Catalogue plat, et non Fluent — tranché le 2026-08-26

**Ce que Fluent apporte** : règles de pluriel CLDR, sélecteurs de genre et
d'accord, formatage des nombres et dates par locale, réutilisation de termes,
et une tolérance aux erreurs — un message mal formé retombe sur sa clef.

**Ce qu'un catalogue plat apporte** : il se lit, se diffe et se relit en revue
sans rien apprendre ; il s'embarque à la compilation, donc **une clef manquante
peut devenir une erreur de build** là où Fluent résout à l'exécution ; et le
test de cohérence entre catalogues est une comparaison d'ensembles de clefs.

**Les chiffres décident.** Sur ~1 700 chaînes à traduire :

| | Nombre |
|---|---|
| Chaînes avec interpolation d'argument | **26** |
| Pluriels | **6** |
| Libellés statiques | ~1 670 |

Tout ce que Fluent fait mieux ne concerne que **32 chaînes, 2 %**. Son coût
s'applique aux 1 700.

**Décision : catalogue plat JSON, plus une convention de pluriel explicite** —
`poules.one` / `poules.other` — et un sélecteur maison pour deux langues.
Français : `n > 1`. Anglais : `n != 1`. Une dizaine de lignes, testées.

Ce qui règle au passage les six pluriels bricolés du code :

```rust
format!("{n} poule{}", if n > 1 { "s" } else { "" })
```

Cette règle est **juste en français et fausse en anglais** — « 0 pools », pas
« 0 pool ». C'est le seul vrai piège linguistique du projet, et il se règle par
une fonction, pas par un moteur.

**L'argument « FTL est le format que les outils comprennent » ne tient pas
ici** : aucun outil ne sait extraire les chaînes d'un template Askama. Ni
`xgettext`, ni les extracteurs Fluent. Quel que soit le format retenu,
**l'extraction sera un script maison** — Fluent n'épargne pas ce travail.

## Les erreurs du domaine — le patron existe déjà

**120 des 869 littéraux français sont dans `domain/`**, dont 40 dans des
`write!`/`format!` de `Display`. Le domaine produit de la prose destinée à
l'utilisateur :

```rust
DomainError::PlayerNameTooLong => "Le nom ne peut pas dépasser 50 caractères.",
```

Et `competitions/domain/error.rs` l'assume : « le message est directement
exploitable comme corps de réponse 422 ».

**La solution la plus simple est déjà dans le code**, `teams/io/web/view_models.rs:99` :

```rust
fn explication_longue(cause: &DomainError) -> &'static str {
    match cause {
        DomainError::StaffNotAllowedForRoster => "Ce roster n'a pas droit à ce personnel.",
        …
    }
}
```

La couche web fait déjà correspondre des variantes à du texte. Généraliser ce
patron n'est pas une nouvelle architecture, c'est étendre celle qui est en
place — et son avantage décisif est qu'il **ne touche pas au domaine du tout**.
Le `Display` reste ce qu'il est et continue de servir au journal, où un message
n'a aucune raison d'être traduit.

L'autre option — le domaine porte une clef de message — a l'air plus directe et
coûte davantage : elle fait entrer une préoccupation de présentation dans la
couche qui doit le moins en connaître, impose de toucher les cinq `error.rs`, et
n'économise aucun catalogue.

**Le volume est borné : 67 variantes.**

| BC | Variantes |
|---|---|
| `team_creation` | 22 |
| `teams` | 20 |
| `match_report` | 15 |
| `players` | 7 |
| `competitions` | 3 |

Seules celles réellement affichées ont besoin d'une entrée. Point de méthode :
**écrire le `match` sans bras `_ =>`**, pour que le compilateur exige une
traduction à chaque nouvelle variante. L'`explication_longue` actuel a un
`_ => "Indisponible pour cette équipe."` — c'est exactement ce qui laisse passer
une variante non traitée sans un mot.

## Architecture pressentie

**La locale voyage par requête**, jamais en variable globale : un middleware la
résout et la pose dans un `tokio::task_local!` pour la durée du handler. Un
`thread_local` serait faux — les tâches async changent de thread.

**Le point dur est Askama**, qui résout tout à la compilation : chaque struct de
template devrait porter la locale, 112 fois. L'échappatoire est un **filtre**
lisant le task-local — `{{ "team.ready"|t }}` — qui évite de toucher les
structs.

Le « à vérifier avant de s'engager » de la première version **a été vérifié** —
voir « Ce que l'écosystème Askama offre » ci-dessus. Le coût est confirmé : un
`use` dans 96 fichiers, une ligne chacun. Et la piste **Fluent y est
abandonnée** au profit d'un catalogue plat.

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

## Tranché le 2026-08-26

- [x] **Instance bilingue** — imposée par les e-mails du cron
- [x] **Catalogue plat JSON** avec convention de pluriel, et non Fluent — son
      apport ne concerne que 2 % des chaînes
- [x] **Filtre Askama** lisant un `task_local`, et non un champ sur les structs
      — coût vérifié : un `use` dans 96 fichiers
- [x] **Erreurs domaine** : `match` exhaustif en couche web, domaine intouché

## Reste à trancher avant `ready_to_be_done`

- [ ] Le sort du corpus de références — un dossier par langue, des libellés
      multilingues par entrée, ou un corpus unique en anglais
- [ ] Si le sélecteur attend la page Mon compte, ou vit d'abord dans le menu
- [ ] Découpage en épic : quelles cartes, dans quel ordre

## État

**En attente, volontairement.** Le chantier a été instruit le 2026-08-26 sans
être ouvert : le volume — ~1 700 chaînes, 96 fichiers à toucher, 67 variantes
d'erreur — le met hors de portée d'une session. Ce qui précède existe pour que
la prochaine ouverture reparte des mesures et non de zéro.
