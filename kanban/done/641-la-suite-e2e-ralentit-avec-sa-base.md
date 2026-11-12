# La suite e2e ralentit avec sa base

**Priorité : haute** — chaque vérification complète coûte désormais une relance,
et un échec réel se noie dans les dépassements de délai
**Trouvée par :** la fusion de la carte 634 (2026-11-02).

## Le symptôme

La suite complète prenait **9 minutes** le 2026-11-01 ; elle en prend **20 à
22** le lendemain, sans changement de périmètre notable. Sur la carte 634, deux
exécutions de suite ont donné deux ensembles d'échecs **différents** :

| exécution | échecs | erreurs | nature |
|---|---|---|---|
| 1 | 11 | 53 | surtout des fixtures |
| 2 | 1 | 11 | `Timeout 30000ms exceeded` — l'assistant de compétition, la page des matchs d'une équipe |

Relancé seul, **chaque fichier en cause passe**. Aucune des pages en délai ne
passe par le code de la 634.

## Le soupçon

La base `kreek_e2e` ne fait que grossir : chaque exécution crée ses
compétitions (`build_full_competition`), ses équipes, ses comptes rendus, et
rien ne les efface. Au 2026-11-02 : 124 Mo, 2387 équipes, 1466 comptes rendus.
Les pages qui listent par espace — calendrier, équipes, assistant — lisent tout
ce qui s'est accumulé dans l'espace de test, qui est **un seul espace** pour
toute la suite.

## La mesure (2026-11-12)

La base `kreek_e2e` a été remise à zéro le 2026-11-12. Même code `demo`,
charge machine comparable :

| base e2e | équipes | durée de la suite complète |
|---|---|---|
| chargée (2026-11-11) | 9 534 | 19 à 22 min |
| fraîche (2026-11-12) | 362 | **8 min 25 s**, 544 réussis |

**Le soupçon est confirmé** : l'écart vient de la base. Elle avait déjà causé
deux incidents : la carte 668, où un sélecteur chargeait 4 060 compétitions,
et la carte 664, où la migration de données `m008` prenait une à deux minutes
au démarrage du serveur.

## Ce qu'il faut faire

1. [x] **Mesurer** : fait, ci-dessus.
2. **Vider la base avant chaque suite complète, dans `make e2e`** :
   `TRUNCATE` de toutes les tables sauf `_sqlx_migrations` et
   `applied_data_migrations`, puis le seed (`scripts/reset_e2e_db.sh`, cible
   `reset_e2e_db`, prérequis de `e2e`). Aucune migration SQL n'insère de
   données : une base vidée puis seedée vaut une base neuve.
3. **Un verrou entre la remise à zéro et pytest** : tous les worktrees
   partagent la base. Chaque session pytest tient un verrou partagé, la remise
   à zéro le demande en exclusif et refuse s'il est pris
   (`tests/e2e/e2e_lock.py`, `fcntl` — la commande `flock` n'existe pas sur
   macOS).
4. **La CI exécute aussi la remise à zéro** : sur une base déjà neuve, elle ne
   change rien au résultat, et c'est le seul endroit qui éprouve ce chemin à
   chaque push. `ci.yml` reste tel quel.
5. **`uv run pytest fichier.py` seul ne vide rien** : une sélection qu'on itère
   garde son contexte, et elle accumule peu.

### Pourquoi `TRUNCATE` l'emporte

Le serveur e2e tourne pendant la remise à zéro, et la règle 8 interdit de le
redémarrer.

| voie | coût | sous le serveur vivant |
|---|---|---|
| **`TRUNCATE` + seed** (retenue) | **4 à 5 s** mesurées | rien n'est détruit : pool, schéma et registres de migrations restent, **aucun redémarrage** |
| `sqlx database reset` + seed | 10 à 30 s | coupe le pool ; `applied_data_migrations` repart vide, et les migrations de données se rejouent au redémarrage suivant (cf. 664) |
| clonage d'une base modèle | < 1 s, plus la reconstruction du modèle | coupe le pool, perd les marques des migrations de données, et il faut détecter un modèle périmé (migrations, seed, registre) |
| fixture pytest de session | comme `reset` | se déclencherait aussi pour un fichier lancé seul |

Le clonage ne gagnerait que quelques secondes sur une suite de huit minutes.
Une première tentative existe : carte 536, commit `0134c07d`, sur la branche
`sondage-presence`, jamais fusionnée. **Le clonage d'une base modèle est
reporté à la carte du parallélisme et des décors pré-construits**, où il fera
gagner des minutes.

## Constaté en passant

`test_draft_team_errors.py::test_submit_without_coach_shows_error`, lancé seul
juste après une remise à zéro, échoue : il attend une compétition dans le
sélecteur, et la base n'en a aucune. Dans la suite complète, il passe parce que
les fichiers `test_competition_*` tournent avant lui. C'est une dépendance à
l'ordre des fichiers, que la base accumulée masquait. Elle relève d'une carte à
part.

## Hors du périmètre

Les requêtes lentes que la mesure révélerait **en production** : une base de
ligue n'accumule pas comme la base de test, mais si une page s'effondre à 2000
équipes, c'est une carte à part, avec ses chiffres.

## Terminé quand

- [ ] deux `make e2e` complets d'affilée, sans rien changer entre les deux,
      durent autant à ±10 % — référence : 8 min 25 s sur base fraîche ;
- [ ] ces deux exécutions donnent le même résultat : mêmes nombres de réussis,
      d'échecs et d'ignorés, et le même ensemble de tests en échec ;
- [ ] le nombre d'équipes en base après chacune reste stable (environ 362), au
      lieu de doubler.

## Conséquences sur les données de production

Aucune : seule la base e2e locale est touchée. Elle est vidée à chaque
`make e2e` ; ce qu'on y aurait créé à la main sur le port 3211 est perdu. La
base de travail (port 3210) n'est jamais touchée, et le script refuse une base
distante ou celle de `.env.dev`.
