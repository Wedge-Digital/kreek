# `seed-accounts` efface le `legacy_id` du compte qu'il sème

> **⚠️ Cette carte demande ton attention.** Elle touche la correspondance entre
> les comptes importés et le système d'origine — une donnée qu'on ne peut pas
> reconstituer une fois perdue autrement qu'en réimportant.

**Priorité : moyenne** — silencieux, et un seul compte concerné aujourd'hui
**Dépend de :** rien
**Trouvée par :** la vérification de la carte du `legacy_id` — ma première
mesure était fausse à cause de ce défaut

## Le constat

`src/cli/seed_accounts.rs` écrit :

```sql
ON CONFLICT (lower(coach_name))
DO UPDATE SET password_hash = EXCLUDED.password_hash,
              email         = EXCLUDED.email,
              legacy_id     = EXCLUDED.legacy_id
```

`legacy_id` est `Option<i32>` et le fichier `scripts/seed_accounts.json` ne le
renseigne pas. `EXCLUDED.legacy_id` vaut donc `NULL`, et l'`UPDATE` **écrase**
la valeur portée par le compte importé.

Mesuré sur la base de développement, après `make init_db` :

| Mesure | Valeur |
|---|---|
| Comptes importés | 852 |
| Comptes ayant conservé leur `legacy_id` | **851** |
| Compte l'ayant perdu | **Bagouze**, le seul que `seed_accounts.json` sème |

`make init_db` enchaîne l'import puis `seed-accounts` : la perte se produit à
chaque initialisation, sans un mot.

## Ce que ça a coûté

En vérifiant que l'import et le seed e2e cohabitaient enfin, j'ai conclu au
succès alors que la valeur `1` avait simplement été **libérée par ce défaut**,
et non par le correctif que je testais. La vérification a dû être refaite avec
`WITH_ACCOUNTS=0` pour que `legacy_id = 1` soit réellement occupé.

Une mesure faussée par un second défaut : c'est le coût réel de celui-ci.

## Ce qui est à trancher

**Ne pas écrire la colonne quand elle n'est pas fournie.** `legacy_id =
COALESCE(EXCLUDED.legacy_id, auth__users.legacy_id)` conserve l'existant et
laisse le seed poser la valeur quand le JSON la porte.

**Ne jamais l'écrire du tout**, comme le fait désormais `seed_e2e.rs` (cf. la
carte du `legacy_id`) : cet espace d'identifiants appartient au système
d'origine, et un seed n'a rien à y revendiquer. Plus net, mais retire au JSON
la capacité d'en poser un.

**Remettre `legacy_id` dans `seed_accounts.json`.** Rétablit la valeur, mais ne
corrige rien : le prochain compte semé sans ce champ effacera le sien.

## Ce que la carte ne couvre pas

**La base de développement actuelle**, où Bagouze a retrouvé son `legacy_id = 1`
par une réinitialisation complète. Rien à réparer en base ; c'est le code qui
reproduit le défaut à chaque `make init_db`.

## Questions à trancher au raffinement

- `seed-accounts` a-t-il vocation à poser un `legacy_id`, ou seulement à donner
  un mot de passe utilisable à un compte déjà importé ?
- Le même `DO UPDATE SET <colonne> = EXCLUDED.<colonne>` écrase-t-il d'autres
  données ailleurs ? `email` est dans le même cas, mais le JSON le renseigne.
- Un test doit-il vérifier qu'un semis par-dessus un compte importé lui laisse
  son `legacy_id` ? `seed_e2e.rs` en a désormais un, qui peut servir de modèle.
