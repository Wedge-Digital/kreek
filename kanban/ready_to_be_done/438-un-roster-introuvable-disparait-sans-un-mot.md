# Un roster introuvable disparaît sans un mot

**Priorité : moyenne-haute** — le défaut ne casse rien, il fait chercher des
heures du mauvais côté
**Périmètre : le BC `team_creation`, chemin de résolution des rosters**
**Dépend de :** rien
**Trouvée par :** un diagnostic de production le 2026-08-27 — un roster ajouté
au corpus n'apparaissait pas dans le sélecteur de création d'équipe

## Le constat

Les rosters proposés à la création d'une équipe sont ceux que les **tiers de la
saison** déclarent, chacun résolu contre le corpus :

```rust
// team_creation/io/web/builders.rs:115
tier.rosters.iter().filter_map(move |uid| {
    ref_data.find_roster_definition(uid).map(|def| RosterPickerItemWithTier { … })
})
```

**`filter_map` laisse tomber en silence** tout uid que le corpus ne résout pas.
Pas de journal, pas d'erreur, pas de trace : le roster n'est simplement pas dans
la liste.

Trois causes produisent le même écran vide, et **rien ne permet de les
distinguer** :

| Cause | Ce qu'on voit |
|---|---|
| Le serveur n'a pas redémarré depuis l'ajout au corpus | rien |
| Le tier ne cite pas ce roster | rien |
| L'uid du tier ne correspond pas à celui du corpus | rien |

Vécu : plus d'une heure de diagnostic pour un roster Slann, en cherchant du côté
du corpus alors que le tier était en cause.

## Ce n'est pas un cas isolé

`roster_service.rs` porte **neuf** occurrences du même motif — `filter_map`
enchaînés à des `.ok()?` sur des smart constructors :

```rust
// roster_service.rs:23-28
.filter_map(|p| {
    Some(Player {
        name: PlayerName::try_new(p.position_name.clone()).ok()?,
        max_quantity: PlayerMaxQuantity::try_new(p.max_quantity).ok()?,
        price: PlayerPrice::try_new(p.cost).ok()?,
        …
```

Un poste dont le nom porte un caractère refusé par le charset **s'évapore du
roster**. Le `CLAUDE.md` recense déjà ce site — « roster escamoté par un
`.ok()?`, deux fois » — sous « Ce que le charset ne règle pas ». Il y en a plus
que deux, et `builders.rs` n'y figure pas.

## Ce que la carte fait

**Elle ne change aucun comportement visible.** Elle rend audible ce qui est
aujourd'hui muet.

### 1. Journaliser chaque élément écarté

```rust
tier.rosters.iter().filter_map(move |uid| {
    let def = ref_data.find_roster_definition(uid);
    if def.is_none() {
        tracing::warn!(
            roster_uid = %uid, tier = %tier_name,
            "roster déclaré par un tier mais introuvable au corpus — écarté du sélecteur"
        );
    }
    def.map(|def| …)
})
```

**`warn` et non `error`** : l'application fonctionne, l'écran s'affiche, et un
uid mort peut être transitoire — c'est exactement ce que produit la suppression
d'un roster personnalisé, le temps que l'app event passe
(`docs/specs/roster-personnalise/`).

**Le message nomme les deux côtés** — l'uid **et** le tier qui le déclare. Sans
le tier, on sait qu'un roster manque mais pas où le chercher.

**Cible `kreek::`, sinon la ligne n'existe pas.** C'est la règle qui prime du
`CLAUDE.md` : le filtre est `kreek=<niveau>,sqlx=warn`, et une cible hors de ce
préfixe n'est activée par aucune directive. Un `tracing::warn!` depuis un module
du projet en relève par construction — mais il faut le vérifier plutôt que le
supposer.

### 2. Les neuf sites de `roster_service.rs`

Même traitement, avec le motif précis : quel champ a refusé quelle valeur.

```
poste écarté du roster : PlayerName::try_new("Piétaille d'élite") a refusé
```

C'est ce qui manquait quand une compétence nommée « Capitaine d'équipe »
échouait sur un `UnknownSkill` accusant le catalogue.

### 3. Un compteur au rendu

Le sélecteur sait combien de rosters le tier déclare et combien il en affiche.
Un écart se dit **à l'écran**, pas seulement au journal :

> 4 rosters sur 5 — un roster déclaré par ce tier est introuvable.

Un ligueur n'ouvre pas `docker logs`. La ligne de journal sert au diagnostic ;
le compteur sert à savoir qu'il y a quelque chose à diagnostiquer.

## Ce que la carte ne fait pas

- **Elle ne refuse rien.** Un tier qui cite un roster mort continue de servir les
  autres. Échouer le rendu punirait tous les coachs pour une erreur de
  configuration.
- **Elle ne corrige aucune donnée.** Les tiers qui citent des uid morts restent
  tels quels ; la carte les rend visibles, à leur administrateur d'agir.
- **Elle ne touche pas au chargement du corpus.**

## Tests

- **Unitaire** : un tier déclarant un uid inconnu produit une ligne `warn` qui
  porte l'uid et le tier, et le sélecteur rend les autres rosters.
- **Unitaire** : un poste dont le nom est refusé par le charset produit une ligne
  qui nomme le champ et la valeur.
- **E2E** : un tier citant un uid mort affiche le compteur « 4 rosters sur 5 ».

Le premier test est le seul qui empêche la régression : rien dans le
compilateur ne signale un `filter_map` redevenu muet.

## Checklist

- [ ] `builders.rs:115` journalise, uid et tier compris
- [ ] Les neuf sites de `roster_service.rs` journalisent leur motif
- [ ] Le compteur au rendu du sélecteur
- [ ] Les trois tests
- [ ] La section « Ce que le charset ne règle pas » du `CLAUDE.md` mise à jour —
      elle annonce quatre sites, il y en a plus
- [ ] `make lint && make test && make check-arch`
