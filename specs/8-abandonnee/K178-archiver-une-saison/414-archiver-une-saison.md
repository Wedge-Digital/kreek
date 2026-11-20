# Archiver une saison

**Priorité : à définir** — rien ne la presse, aucune ligue n'a demandé à ranger
ses saisons finies
**Statut : à raffiner**
**Née de :** l'onglet Paramètres (`docs/specs/modifier-une-competition/`), dont
la maquette portait une « zone de danger » — retirée le 2026-08-26

## La décision qui la précède

**On n'implémentera pas de suppression de saison.** Une saison finie
s'**archive** : elle sort des affichages courants, et rien n'est détruit.

Ce n'est pas de la prudence de principe, c'est le coût mesuré. Huit tables
portent `season_id`, sur quatre BCs, sans compter `competition_seasons`
elle-même :

```
competition_groups · competition_match_days · competition_match_display_proj
competition_notification_deliveries · match_report_proj · ranking_lines
team_drafts · team_proj
```

(`competition_group_teams` suit par cascade ; `competitions_members` s'attache
à la **compétition**, pas à la saison, et survivrait donc à l'archivage.)

**Et les trois flux d'événements n'en portent pas.** `team_event_store`,
`players_events` et `match_report_event_store` ne connaissent que leur agrégat.
Les atteindre imposerait de passer par les projections pour savoir quoi
détruire — se servir d'un dérivé rebuildable comme index de la destruction, à
rebours du sens de lecture.

Une équipe appartient à une saison (`team_proj.season_id`) : supprimer la saison
détruirait les équipes, leurs joueurs et toute leur histoire.

*Réinitialiser* — effacer les résultats en gardant les équipes — n'est pas la
version douce du même bouton : les équipes garderaient les SPP, les blessures,
les valeurs et les trésoreries gagnés dans des matchs qui n'existent plus.

## Ce que l'archivage doit faire

**Filtrer l'affichage**, et c'est l'essentiel. Mais pas seulement :

**Il doit aussi taire le cron.** Deux requêtes alimentent l'envoi automatique
d'e-mails — `list_seasons_with_deadline.sql` et
`list_seasons_with_round_closing.sql`. Sans filtre, une saison archivée
continuerait de relancer ses coachs sur des échéances d'une saison rangée.
C'est le seul effet de l'archivage qui ne se voit pas à l'écran, et donc le
seul qu'on peut oublier.

## Où poser le drapeau

`competition_seasons.status` existe déjà, avec cinq valeurs :

| `draft` | `structure_selected` | `rules_selected` | `invitations_configured` | `ready` |
|---|---|---|---|---|
| 163 | 214 | 233 | 208 | 814 |

**Mais il porte l'avancement du magicien, pas le cycle de vie.** Une saison
archivée reste `ready` — elle l'a été. Ajouter `archived` à cette colonne
fondrait deux axes et ferait perdre l'information d'origine.

Piste : une colonne `archived_at timestamptz NULL` — nulle par défaut, qui dit
*quand* plutôt que *si*, et rend le désarchivage trivial.

## Les endroits à filtrer

| Requête | Ce qu'elle sert |
|---|---|
| `competitions/find_by_space_id.sql` | la liste des compétitions d'un espace |
| `competitions/find_competitions_with_seasons.sql` | la même, avec les saisons |
| `seasons/find_latest_season_id.sql` | la « saison courante » — prendrait une archivée sinon |
| `notifications/list_seasons_with_deadline.sql` | le cron d'échéance |
| `notifications/list_seasons_with_round_closing.sql` | le cron de clôture de journée |

Liste à confirmer au raffinage : elle a été établie par lecture des requêtes qui
listent, pas par un parcours exhaustif.

## À trancher avant de passer en `ready_to_be_done`

- [ ] Qui archive — admin de compétition, admin d'espace, les deux ?
- [ ] Une saison archivée reste-t-elle **consultable** par lien direct, ou
      devient-elle inaccessible ? (consultable est le sens du mot « archive »)
- [ ] Le désarchivage existe-t-il, et sous quelle forme ?
- [ ] Où vit le bouton : onglet Paramètres, ou liste des compétitions ?
- [ ] Peut-on archiver une saison qui n'est pas finie — et qu'est-ce que « finie »
      veut dire, alors qu'aucun statut ne la marque aujourd'hui ?
- [ ] La compétition elle-même s'archive-t-elle, ou seulement ses saisons ?
