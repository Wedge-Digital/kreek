# `match_report` — les journaliers d'un match sans coups de pouce

**Priorité : à mesurer** — défaut supposé, pas encore constaté
**Trouvée par :** l'analyse d'impact des maquettes mobiles (2026-11-01). Sortie
de E19 : elle ne relève pas de la saisie mobile.
**Fichiers :** `src/app/match_report/use_cases/record_fan_factor_use_case.rs`,
`src/app/match_report/use_cases/record_inducements_use_case.rs`,
`src/app/match_report/use_cases/init_temp_players_use_case.rs`

## Le soupçon

Les joueurs temporaires d'un match — journaliers, mercenaires, vedettes — sont
créés par `init_temp_players_use_case`, et ce use case n'a qu'un appelant :
`record_inducements_use_case`, c'est-à-dire l'enregistrement des coups de pouce.

Or, quand le tier de la compétition ne propose aucun coup de pouce,
`record_fan_factor_use_case` (l. 144-150) redirige l'étape 2 directement vers le
match, sans passer par cet écran. Si rien d'autre ne crée les temporaires, une
équipe à moins de onze joueurs alignables joue sans ses journaliers : on ne
peut pas saisir leurs actions, et ils ne gagnent pas de SPP.

## Ce qu'il faut faire

1. **Mesurer d'abord.** Sur la base de prod, en lecture seule et en ne sortant
   que des totaux (règle 16) : combien de comptes rendus — publiés et en cours —
   dans des compétitions sans coups de pouce, pour une équipe qui aurait eu
   droit à des journaliers. Si le total est nul et qu'aucune compétition n'est
   dans ce cas, la carte se ferme sur ce constat.
2. **Reproduire par un test** : tier sans coups de pouce, équipe à dix
   alignables, étape 2 enregistrée — les journaliers existent-ils au match ?
3. **Corriger si le défaut est confirmé** : initialiser les temporaires sur le
   chemin qui saute les coups de pouce.

## Conséquences sur les données existantes

- **Matchs déjà publiés** : joués sans journaliers en base, ils ne se
  reconstituent pas — aucune action de journalier n'a pu être saisie. Ils
  restent tels quels.
- **Comptes rendus en cours, déjà passés par l'étape 2** : la correction ne les
  rattrape que si elle crée aussi les temporaires à l'ouverture du match. À
  décider avec le résultat de la mesure.
