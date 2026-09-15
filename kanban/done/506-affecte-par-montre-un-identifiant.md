# « Affecté par » montre un identifiant, pas un nom

**Priorité : basse** — l'information est juste, elle est illisible
**Dépend de :** rien · **Sans épic**
**Signalée par :** l'utilisateur

> **Faite par la carte 548**, commit `2ab5e292` — écrite en double le
> 2026-09-15 faute d'avoir cherché si une carte décrivait déjà le défaut. Les
> deux restent en `done/` plutôt qu'une en `cancelled/` : le numéro 548 est
> dans l'historique git, et un doublon se documente mieux qu'il ne s'efface.
> C'est **cette carte-ci** qui porte la conception ; la 548 porte le récit de
> la livraison.

## Le constat

La liste des points de classement manuels affiche, sous « Affecté par », une
chaîne de vingt-six caractères au lieu du nom du commissaire.

Vérifié en base : les trois lignes de `ranking__manual_points` portent toutes
`01M0RFRN10HXV4Q1HTRGSCTFER`, l'identifiant du coach **Bagouze**.

## La cause

L'identifiant traverse toute la pile sans que rien ne le traduise :

```
membre(&auth_session)  →  u.id.to_string()          ← l'identifiant
        ↓
cmd.user_id            →  insert_manual_points(…, &cmd.user_id)
        ↓
awarded_by (colonne)   →  row.awarded_by  →  {{ line.awarded_by }}
```

`membre()` ne retient que l'identifiant, ce qui est **correct pour son autre
usage** — l'autorisation. `User` porte pourtant `coach_name` juste à côté.

Un indice de l'intention d'origine : `ranking_repository.rs:956` teste
`awarded_by == "DevCoach"`, un **nom**. Il passe parce qu'il insère lui-même
cette valeur. La colonne a été pensée pour un nom, et le handler y écrit un
identifiant.

## La décision : l'identifiant en base, le nom à l'affichage

**On garde l'identifiant.** C'est lui qui désigne la personne de façon stable :
un coach renommé reste le même commissaire, et l'historique ne ment pas.

**On résout au moment d'afficher.** `ranking` ne copie pas une donnée qui
appartient à `auth` — il la consulte par port, exactement comme il le fait déjà
pour l'autorisation avec `IRankingAdminPort`. C'est la souveraineté des données
du CLAUDE.md.

**Aucune reprise de données** : les trois lignes existantes s'afficheront
correctement, puisqu'elles portent déjà le bon identifiant.

L'autre voie — écrire le nom à l'insertion — tenait en une ligne, mais figeait le
nom du jour et laissait les trois lignes existantes illisibles.

## Le précédent qu'on suit

`match_report` a déjà ce port, au mot près :

```rust
pub trait ICoachDataPort: Send + Sync {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String>;
}
```

Son adapter (`infrastructure/match_report/coach_data_adapter.rs`) passe par
`ISpaceUserCacheRepository`. `ranking` reçoit le sien, construit sur le même
dépôt — **le code de l'adapter est copié, pas réécrit** (règle 5).

Deux BCs consommateurs, deux ports : c'est la règle des adapters inter-BCs. Un
port partagé les coupleraient l'un à l'autre.

## Ce qu'on affiche quand le nom manque

Un identifiant peut ne plus correspondre à personne — compte supprimé, donnée
héritée. Le port rend `Option`, et l'écran affiche alors **« Commissaire
inconnu »** plutôt qu'un identifiant nu ou une cellule vide : la première forme
n'apprend rien au lecteur, la seconde lui laisse croire à un défaut d'affichage.

## Ce que la carte ne fait pas

**Elle ne touche pas à l'insertion.** `awarded_by` continue de recevoir
l'identifiant.

**Elle ne corrige pas le test de `ranking_repository`**, qui insère « DevCoach »
comme valeur arbitraire — il teste l'aller-retour SQL, pas la sémantique du
champ. Son intention mérite un commentaire, pas une réécriture.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `un_identifiant_connu_devient_un_nom` | la traduction, unitaire |
| `un_identifiant_inconnu_affiche_commissaire_inconnu` | le repli |
| `test_la_liste_montre_le_nom_du_commissaire` | l'écran, e2e — et **pas** un identifiant de vingt-six caractères |

## Checklist

- [x] `ICoachDataPort` dans `ranking/ports.rs`
- [x] Son adapter dans `infrastructure/ranking/`, copié de `match_report`
- [x] Injecté dans le contexte du BC par `main.rs`
- [x] Le builder de la liste traduit, avec repli
- [x] Les trois tests, chacun falsifié
- [x] `make lint && make test && make check-arch && make e2e`

Un quatrième test s'est ajouté à la livraison, que cette carte n'avait pas
prévu : `resolve_coach_names` n'interroge le port qu'**une fois par identifiant
distinct**. Une journée de forfaits, ce sont vingt lignes du même commissaire,
et le dédoublonnage ne se voit nulle part dans le résultat — seul un port qui
compte ses appels peut le prouver.
