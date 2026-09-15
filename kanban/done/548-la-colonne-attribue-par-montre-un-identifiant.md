# La colonne « Attribué par » montre un identifiant

**Priorité : moyenne — visible en production, sans perte de donnée**
**Épic :** aucune — finition de la carte 452

> **Doublon de la carte 506**, qui décrivait le même défaut et attendait en
> `ready_to_be_done`. Écrite sans l'avoir cherchée : seul le prochain numéro
> libre avait été relevé, pas l'existence d'une carte sur le sujet. La 506
> porte la conception, celle-ci le récit de la livraison ; le code est commun,
> commit `2ab5e292`.
**Fichiers :** `src/app/ranking/ports.rs`,
`src/app/ranking/context.rs`,
`src/infrastructure/ranking/mod.rs`,
`src/infrastructure/ranking/coach_data_adapter.rs` *(déjà écrit, non suivi)*,
`src/app/ranking/io/web/manual_points/builders.rs`,
`src/app/ranking/io/web/manual_points/view_models.rs`,
`src/app/ranking/io/web/manual_points/controller.rs`,
`src/app/ranking/io/web/templates/manual-points/list.html`,
`src/main.rs`,
`tests/e2e/test_manual_ranking_points.py`

## Le symptôme

Sur la page de gestion des points manuels, la colonne **« Attribué par »**
affiche un ULID brut — `01K4F2M8QX7W…` — au lieu du nom du commissaire.

## La cause

L'identifiant traverse toute la chaîne sans jamais être résolu :

```
controller.rs:83    Some(u) => Ok(u.id.to_string())    ← l'id, pas coach_name
        ↓            AwardManualPointsCommand.user_id
repository          INSERT … awarded_by
builders.rs:91      awarded_by: row.awarded_by         ← passé tel quel
list.html:68        <td>{{ line.awarded_by }}</td>
```

`auth::domain::User` porte `id: UserId` **et** `coach_name: CoachName` comme
deux champs distincts. Le contrôleur prend le premier, et rien plus loin ne
sait retrouver le second.

**On ne touche pas à ce qui est stocké.** `awarded_by` garde l'identifiant :
c'est la bonne clé, stable, insensible à un changement de pseudonyme. C'est
l'affichage qui doit résoudre, pas la base qui doit dénormaliser.

## Le travail est à moitié fait, et ne compile pas

`src/infrastructure/ranking/coach_data_adapter.rs` **existe déjà sur le
disque**, écrit le 4 septembre 2026, et n'est pas suivi par git. Il fait
exactement ce qu'il faut — un copier-coller de
`infrastructure/match_report/coach_data_adapter.rs`, seul l'import du port
changeant, conformément à la règle 5.

Il ne compile pas, et ne peut pas :

| | |
|---|---|
| `ICoachDataPort` dans `ranking/ports.rs` | **absent** — n'existe que dans `match_report` |
| déclaré dans `infrastructure/ranking/mod.rs` | **non** |
| câblé dans `main.rs` / `RankingContext` | **non** |

N'étant dans aucun `mod.rs`, `cargo build` ne le voit pas : l'arbre reste vert
avec un fichier qui référence un trait inexistant. C'est ce qui a permis à ce
chantier de dormir onze jours sans que rien ne le signale.

Son en-tête annonce « carte 500 ». Ce n'était pas une faute : **la carte 506 a
porté le numéro 500** avant d'être renumérotée, 500 étant déjà pris par le
bandeau de la fiche d'équipe. La référence était périmée, pas fausse — et je
l'ai d'abord prise pour une erreur, faute d'avoir ouvert la 506. Elle cite
désormais les deux cartes.

## Ce qu'il faut faire

1. **Le port** — `ICoachDataPort` dans `ranking/ports.rs`, même forme que
   celui de `match_report` : `async fn find_coach_name(&self, coach_id: &str)
   -> Option<String>`.
2. **L'adapter** — déclarer celui qui existe dans
   `infrastructure/ranking/mod.rs`, et corriger sa référence de carte.
3. **Le contexte** — `coach_data: Arc<dyn ICoachDataPort>` dans
   `RankingContext`, instancié dans `main.rs` sur
   `SpaceUserCacheRepository`, comme `match_report` ligne 488.
4. **La résolution** — une fonction `resolve_coach_names` dans `builders.rs`,
   qui résout les identifiants **distincts** d'un relevé en une passe et rend
   une table `id → nom`. Une page de vingt lignes attribuées par le même
   commissaire ne doit pas faire vingt requêtes.
5. **Le VM** — `awarded_by: Option<String>`, comme `reason` juste au-dessus et
   comme `submitted_by` dans `match_report`. `build_teams` reste **synchrone**
   et reçoit la table : c'est le contrôleur qui `await`, là où vivent déjà les
   autres appels de port.
6. **Le gabarit** — un tiret quand le nom ne se résout pas.

### Le repli, et pourquoi un tiret

Un compte supprimé ne se résout plus. Trois issues possibles :

| | |
|---|---|
| garder l'ULID | c'est le défaut qu'on corrige |
| un tiret | se lit comme un défaut d'affichage |
| **« Commissaire inconnu »** | dit qu'on a cherché et pas trouvé |

Livré d'abord avec un tiret, au motif qu'un gabarit n'invente aucune valeur.
La règle vise une valeur **plausible** qu'on ne saurait plus distinguer d'une
vraie — un chiffre, un nom. « Commissaire inconnu » n'en est pas une : c'est un
libellé d'absence, et il dit ce que le tiret laissait deviner. La 506 avait
tranché ainsi avant moi.

## Terminé quand

Sur la page de gestion des points manuels d'une saison, la colonne
« Attribué par » d'une ligne qu'on vient d'attribuer affiche **le pseudonyme du
coach connecté**, et aucune cellule de la colonne ne contient de chaîne de 26
caractères alphanumériques.

## Tests

- **Unitaire** (`builders.rs`) — `to_vm` rend `Some(nom)` quand la table
  contient l'identifiant, `None` quand elle ne le contient pas ; et
  `resolve_coach_names` n'interroge le port qu'**une fois par identifiant
  distinct**, vérifié par un port bouchon qui compte ses appels.
- **E2E** (`test_manual_ranking_points.py`) — après attribution, la cellule
  « Attribué par » porte le pseudonyme du coach, et ne correspond pas à
  `^[0-9A-Z]{26}$`. Aucun test e2e ne regardait cette colonne : c'est pourquoi
  le défaut est passé.
