# Les destinataires, et ceux sans adresse

**Priorité : moyenne — sans lui, les DTOs de port remonteraient jusqu'aux gabarits**
**Épic :** E16 — Sondage de présence
**Dépend de :** 510, 511
**Fichiers :** `src/app/competitions/use_cases/presences/survey_roster_service.rs`

## L'objectif

Croiser les équipes engagées, les adresses des coachs et les réponses de la
campagne, et rendre des **objets du domaine local**.

C'est le cas d'école de la section « Domain services pour données inter-BCs » du
CLAUDE.md : `TeamInfoDto` et `SpaceMemberDto` **n'atteignent jamais un handler
ni un gabarit**.

## Conception

| Besoin | Port | Ce qu'il rend |
|---|---|---|
| les équipes engagées de la saison | `ITeamInfoPort::find_enrolled_teams` | `team_id`, `team_name`, `coach_id`, `coach_name`, `logo_url` |
| l'adresse des coachs | `ICompetitionSpaceMemberPort::list_space_members` | `coach_id`, `coach_name`, `email` |

**Aucun port à créer, aucun adapter à écrire** — les deux existent et sont déjà
dans le contexte.

`TeamInfoDto` ne porte pas l'adresse : c'est le croisement des deux listes sur
`coach_id` qui produit les destinataires, et **son défaut de correspondance qui
produit le compte « sans adresse connue » de R3**. Un coach sans adresse
n'empêche pas le lancement — son équipe entre d'emblée dans la colonne « sans
réponse », avec sa mention. Refuser le lancement ferait dépendre une campagne de
quatorze coachs de la fiche incomplète d'un seul.

**`coach_label` est construit ici**, pas dans le gabarit : « Lepandawan · 2
équipes » suppose de savoir combien d'équipes ce coach engage dans cette saison,
ce qui est une question sur le roster de la campagne, pas sur la ligne affichée.

## Checklist

- [x] Le croisement des deux ports sur `coach_id`
- [x] Le compte des sans-adresse (R3)
- [x] `coach_label` avec le nombre d'équipes du coach
- [x] La composition avec les réponses de la campagne, pour les trois colonnes
- [x] `// arch:no-instrument` sur `charger`
- [x] **`PresenceSurvey::absents()`** — hors périmètre, entraîné par le §5
- [x] Tests unitaires : les trois de la liste, plus l'adresse vide, le coach à
      une seule équipe, les trois colonnes et la réponse orpheline — 7 tests
- [x] `make lint`, `make check-arch`, `make test` — 1800/1800

## Ce que la réalisation a tranché

**L'agrégat classe, le service joint.** `colonnes` demande les trois listes à
`presents()`, `absents()` et `sans_reponse()`, et n'ajoute que ce que la campagne
ne connaît pas — le nom d'équipe et le libellé du coach. Refiltrer sur `Presence`
dans la couche applicative aurait fait dire R5 à deux endroits, et le jour où la
règle bouge l'un des deux serait resté en arrière.

Conséquence directe : **`PresenceSurvey::absents()` manquait**. L'agrégat avait
`presents()` et `sans_reponse()` mais seulement `compte_absents()`. Ajoutée,
symétrique des deux autres, et `compte_absents()` passe désormais par elle.

**Une adresse vide vaut une absence d'adresse.** `SpaceMemberDto.email` est un
`String` et non un `Option` : le cas se règle donc ici, par un `trim().is_empty()`.
Sans quoi un coach à l'adresse vide serait compté « avec adresse » et l'envoi
échouerait sans que R3 l'ait annoncé.

**Une réponse dont l'équipe a quitté la saison garde sa ligne**, sous le libellé
« Équipe désengagée ». L'effacer ferait disparaître une réponse que R18 écartera
de toute façon du tirage, avec son motif — et une ligne muette vaut mieux qu'une
disparition.

## Ce qui est reporté à la carte 520

**Les initiales de l'avatar.** `AnswerRowVm` en porte, et la maquette montre
« CC » pour « Les Crocs du Chaos » et « EN » pour « Étoiles de Naggaroth » : les
mots-liens y sont sautés, ce que `initials_from` de `teams/domain/team.rs` ne
fait pas — il donnerait « LC » et « ÉD ». C'est une décision de mise en forme :
si le résultat était faux, on corrigerait la vue. Elle se prend avec l'écran, où
l'avatar vit. `LignePresence` porte `team_name`, donc rien ne bloque.
