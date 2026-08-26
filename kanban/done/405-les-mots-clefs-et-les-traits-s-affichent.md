# Les mots-clefs et les traits s'affichent

**Priorité : moyenne**
**Dépend de :** 399 (le corpus porte les `keywords`) et 403 (le mode `Injury` existe)
**Maquettes :** `assets/rawpages/html/app-team-detail-keywords.html`,
`app-player-detail-haine.html`
**Fichiers :** `src/app/players/ports.rs`,
`src/infrastructure/players/skill_catalog_adapter.rs`,
`io/web/widgets/player_table_widget.rs`, `io/web/player_detail_controller.rs`,
`io/app_events/team_created_listener.rs`,
`io/web/templates/{player-table-fragment,player-detail}.html`,
`assets/static/css/…`

## Objectif

Trois affichages, aucune donnée nouvelle à aller chercher :

1. les **mots-clefs du poste** sous le poste, dans le tableau de la fiche
   d'équipe et dans l'en-tête de la fiche joueur ;
2. la **Haine** distinguée des autres compétences par sa couleur de catégorie ;
3. le **mode d'acquisition** `Injury` traduit « Blessure » au journal des
   évolutions.

Cette carte remplace les phases 2 à 7 du workflow pour ces deux pages : il ne
s'agit que d'afficher des informations déjà présentes, sur des écrans qui
existent.

## Les mots-clefs n'exigent aucune requête

`player_table_widget` appelle **déjà** `catalog.find_position(&p.roster_line_id)`
pour résoudre les compétences de base. Il suffit d'ajouter le champ au DTO :

```rust
pub struct PositionCatalogEntryDto {
    // …
    pub keywords: Vec<String>,   // rempli par skill_catalog_adapter
}
```

Les libellés viennent du corpus, résolus au même endroit — `players` a déjà tout
ce qu'il faut.

## Du texte, jamais un badge

```css
.player-keywords {
  font-size: 11px; font-style: italic; font-weight: 400;
  color: var(--dark-3); line-height: 1.4;
}
```

« Elfe, Blitzer », en italique gris, sous le badge de poste. **Pas de pastille** :
trois familles en cohabitent déjà dans le tableau des joueurs — poste,
compétences de base, compétences acquises — et une quatrième disputerait l'œil au
poste, que les mots-clefs ne font que qualifier.

Une seule règle CSS pour les deux écrans, une seule classe.

## Le `match` des catégories est faux, et on l'ouvre

`skill_category_css` (`team_created_listener.rs:37`) :

```rust
"GENERAL" => "type-general",
"STRENGTH" => "type-strength",
"AGILITY" => "type-agility",
"PASSING" => "type-passing",
"MUTATION" => "type-mutation",     // ← le corpus dit MUTATIONS
_ => "type-general",               // ← DEVIOUS et TRAITS tombent ici
```

Le corpus déclare sept catégories : `GENERAL`, `AGILITY`, `STRENGTH`, `PASSING`,
`DEVIOUS`, `MUTATIONS`, `TRAITS`. La fonction teste `MUTATION` au **singulier**,
et ignore `DEVIOUS`.

**Conséquence, aujourd'hui, en production** : les compétences de mutation et les
retors s'affichent avec la couleur du général. Personne ne l'a vu parce qu'une
couleur fausse ne casse rien — elle ment, simplement.

La Haine oblige à ouvrir ce `match` ; autant le réparer là plutôt que de le
laisser un an de plus. Les trois entrées manquantes rejoignent les cinq
existantes, et le repli `_` reste, pour une catégorie qu'un corpus futur
inventerait.

**Le violet est déjà pris** : `.type-mutation` vaut `#6f35a5`. `TRAITS` a donc
besoin d'une autre teinte, sinon un trait et une mutation deviennent
indiscernables — ce que la maquette n'avait pas vu. À choisir à
l'implémentation, dans les tokens de `common.css`.

Le gain dépasse la Haine : « Lourdaud » et « Capitaine d'équipe », déjà présents
dans le corpus, cesseront de se déguiser en compétences générales.

## Le journal des évolutions

`AcquisitionMode::Injury` s'affiche **« Blessure »**, comme `Chosen` s'affiche
« Choisie ». Coût et valeur restent à `—` : ni `0 SPP`, ni `+0 kPo`. Le projet a
déjà tranché ce point pour la customisation, et l'affichage doit dire la même
chose que le modèle — un zéro affiché invite à croire qu'un calcul a eu lieu.

## Checklist

- [ ] `PositionCatalogEntryDto.keywords`, rempli par `skill_catalog_adapter`
- [ ] `PlayerRowVm` porte les mots-clefs ; tableau de la fiche d'équipe
- [ ] En-tête de la fiche joueur
- [ ] Une seule règle `.player-keywords`, dans une feuille **inscrite au bundle**
- [ ] `skill_category_css` : `MUTATIONS` corrigé, `DEVIOUS` et `TRAITS` ajoutés
- [ ] Teinte propre à `TRAITS`, distincte du violet des mutations
- [ ] `AcquisitionMode::Injury` → « Blessure » au journal, coût et valeur à `—`
- [ ] Tests unitaires :
  - [ ] un poste sans mot-clef n'affiche pas de ligne vide
  - [ ] `skill_category_css` sur les **sept** catégories du corpus, plus une
        inconnue qui doit retomber sur le général
- [ ] Test e2e : un joueur affiche ses mots-clefs sur les deux écrans, et sa
      Haine parmi ses compétences avec la couleur des traits
- [ ] `make lint`, `make check-arch`, `make test`
