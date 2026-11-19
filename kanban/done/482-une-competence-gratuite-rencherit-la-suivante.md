# Une compétence gratuite renchérit la suivante

**Priorité : haute** — surfacturation silencieuse, 88 joueurs armés en production
**Dépend de :** rien · **Sans épic**
**Trouvée par :** l'utilisateur, en lisant le coût affiché d'une compétence

## Le constat

`players/domain/player.rs:894`

```rust
pub fn next_improvement_level(&self) -> u8 {
    ((self.acquired_skills.len() + self.stat_increases.len()) as u8 + 1).min(6)
}
```

Le niveau — celui qui indexe la matrice de coût en SPP — est la **longueur de la
liste des compétences acquises**, sans aucun filtre sur le mode d'acquisition.
Or `acquired_skills` reçoit quatre choses, dont deux qui ne sont pas des
améliorations :

| Événement | Mode | Coûte des SPP | Ajoute de la valeur | Compte dans le niveau |
|---|---|---|---|---|
| `InitialSkillEarned` | `Chosen`/`Random` | oui | oui | oui — légitime |
| `PlayerSkillPurchased` | `Chosen`/`Random` | oui | oui | oui — légitime |
| `PlayerHatredGained` | `Injury` | **non** | **non** | **oui** ← |
| `PlayerSkillCustomised` | `Customised` | **non** | **non** | **oui** ← |

Les deux dernières sont posées avec `spp_cost: 0` et `value_delta: 0`, sous le
commentaire *« une compétence donnée par un commissaire ne se paie pas et ne
renchérit pas le joueur »*. Elle ne le renchérit pas, en effet — elle renchérit
**la suivante**.

## Ce qui rend le défaut certain plutôt que discutable

**La même décision a été prise correctement pour les caractéristiques.** Une
caractéristique customisée va dans une liste séparée :

```rust
pub acquired_skills:     Vec<AcquiredSkill>,      // les customisations y entrent
pub stat_increases:      Vec<StatIncrease>,       // comptées
pub stat_customisations: Vec<StatCustomisation>,  // séparées, donc non comptées
```

`next_improvement_level` additionne les deux premières et ignore la troisième.
Une caractéristique offerte ne fait donc pas monter le niveau ; une compétence
offerte, si. **La différence tient au seul choix de la liste où l'on pousse**,
pas à une règle.

La spec le confirme : `docs/specs/player-customisation/player-detail/06-domaine.md`
détaille l'asymétrie voulue — ni valeur d'équipe, ni coût — sans jamais
mentionner le niveau. La question n'a pas été tranchée dans le mauvais sens ;
elle n'a pas été posée.

## Ce que ça coûte

Les deux consommateurs lisent la même valeur : `spp_spending_widget.rs:86` pour
**l'affichage du prix**, `purchase_skill_use_case.rs:37` pour **le débit réel**.
Le coach voit le mauvais prix, et il le paie.

Sur le barème, pour une compétence choisie primaire standard :

| Compétences gratuites | Niveau affiché | Niveau juste | Demandé | Juste |
|---|---|---|---|---|
| 1 | 2 | 1 | 8 SPP | 6 |
| 2 | 3 | 1 | 12 | 6 |
| 3 | 4 | 1 | 16 | 6 |

## En production, le 2026-08-30

```
108 customisations de compétence · 6 Haines
 88 joueurs portent au moins une compétence gratuite
      66 à +1 niveau · 18 à +2 · 4 à +3
  3 joueurs ont acheté en SPP
  0 de ces trois n'avait de compétence gratuite
```

**Personne n'a encore surpayé** — par chance. Mais 88 joueurs sont armés : le
prochain achat sur l'un d'eux sera surfacturé.

## La décision

**Ni la customisation ni la Haine ne comptent.** La Haine est gratuite comme une
customisation ; qu'elle vienne du jeu et non d'un commissaire ne change rien au
fait qu'elle n'a rien coûté.

## La correction

Un prédicat sur `AcquisitionMode`, en `match` **exhaustif et sans joker** :

```rust
impl AcquisitionMode {
    pub fn est_une_amelioration(self) -> bool {
        match self {
            Self::Chosen | Self::Random => true,
            Self::Customised | Self::Injury => false,
        }
    }
}
```

Un `_ =>` le supprimerait en silence : c'est l'idiome déjà employé par
`Team::treasury_movement`, dont le commentaire dit exactement pourquoi. Un
cinquième mode d'acquisition cassera la compilation, et son auteur devra
trancher.

## Rien à faire pour l'existant

Le niveau est **dérivé** de l'event store, jamais stocké : la correction
s'applique d'elle-même au prochain rejeu. Aucune migration, aucun script de
reprise — et rien à rembourser, personne n'ayant surpayé.

## Ce que rien ne gardait

Le seul test de la méthode —
`next_improvement_level_counts_skills_and_stats_together_capped_at_6` —
n'emploie que des achats `Chosen` et des augmentations de caractéristique.
**Aucun test n'y met une compétence customisée ni une Haine.** Le comportement
n'était pinné dans aucun sens.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `une_competence_customisee_ne_fait_pas_monter_le_niveau` | la décision |
| `une_haine_ne_fait_pas_monter_le_niveau` | son jumeau |
| `une_competence_achetee_fait_monter_le_niveau` | la contre-épreuve, sans laquelle le premier passerait sur un niveau figé à 1 |
| `le_niveau_melange_achats_et_caracteristiques_en_ignorant_les_gratuites` | les quatre origines ensemble |
| `le_prix_affiche_ignore_les_competences_gratuites` | de bout en bout, par le service de coût |

## Checklist

- [x] `est_une_amelioration` sur `AcquisitionMode`, `match` sans joker
- [x] `next_improvement_level` filtre par ce prédicat
- [x] Six tests unitaires et trois e2e, chacun falsifié
- [x] `make lint && make test && make check-arch` — et `make e2e`

---

# Ce que la réalisation a appris

## Le verrou promis a été vérifié, pas supposé

La carte annonce qu'un cinquième mode d'acquisition cassera la compilation.
Vérifié en ajoutant une variante d'essai : six `match` refusent alors de
compiler, dont celui d'`est_une_amelioration` (`player.rs:78`). Les cinq autres
avaient déjà la même discipline — c'est cohérent avec le reste de ce BC.

## Trois couches, une seule corrigée, trois à garder

Le niveau est lu à trois endroits, et chacun a désormais son test :

| Couche | Ce qui le lit | Test |
|---|---|---|
| Domaine | `next_improvement_level` | 4 tests, dont le mélange des quatre origines |
| Service de coût | `resolve_skill_cost(…, level)` | `le_prix_affiche_ignore_les_competences_gratuites` |
| Widget | `skill_picker_url` porte `level=N` | `une_competence_gratuite_ne_fait_pas_monter_le_niveau_du_selecteur` |

Le troisième a compté : le niveau est **baké dans l'URL du sélecteur**, donc il
tarife *toutes* les lignes du picker, pas seulement celle qu'on achète. Un
joueur customisé se voyait proposer l'écran entier au tarif du niveau supérieur.

**Remettre le défaut d'origine — retirer le filtre — fait rougir les cinq tests
de bout en bout.** C'est ce qui manquait : aucun test ne combinait une
compétence gratuite et un achat, et c'est exactement dans ce trou que le défaut
vivait.

## Le test e2e, et pourquoi son assertion est différentielle

`tests/e2e/test_niveau_apres_customisation.py`, dans un fichier à lui — les deux
fixtures candidates sont partagées à l'échelle du module, et y greffer un joueur
customisé aurait demandé de la chirurgie sur un montage dont six autres tests
dépendent.

Deux joueurs de la même équipe, tous deux crédités par le même match, tous deux
sans achat : l'un a reçu une compétence d'un commissaire, l'autre non. **Leur
tarif affiché doit être identique.**

Comparer plutôt qu'affirmer « 6 SPP » met le test à l'abri du barème : si la
matrice de coût change un jour, il continue de vérifier ce qu'il prétend — que
le cadeau ne déplace rien — au lieu d'échouer sur un chiffre qui n'était pas son
sujet. La valeur est vérifiée en second, pour que l'échec sache dire « le barème
a changé » plutôt que « le cadeau compte ».

Trois scénarios : l'égalité des tarifs, la contre-épreuve du montage — la
compétence offerte est bien là et n'a rien coûté — et celle du barème, un achat
réel devant faire monter le tarif. Sans la dernière, les deux premières
passeraient sur un sélecteur figé au niveau 1 pour tout le monde.

Falsifié : le défaut d'origine remis, le test rougit sur
« le cadeau a déplacé le tarif : 8 SPP contre 6 SPP pour un joueur qui n'a rien
reçu » — le symptôme exact que l'utilisateur avait décrit.

## Falsification

| Mutation | Constaté |
|---|---|
| Le filtre retiré — le défaut d'origine | 5 rouges, sur les trois couches |
| La customisation redevient une amélioration | 4 rouges |
| La Haine redevient une amélioration | 3 rouges |
| Un achat cesse de compter | 6 rouges, dont deux tests préexistants |
| Une cinquième variante de mode ajoutée | erreur de compilation, `player.rs:78` |
| Le filtre retiré, vu du navigateur | l'e2e rouge, en nommant le symptôme |
