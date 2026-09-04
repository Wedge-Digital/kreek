# La correction était jugée avant que les équipes aient bougé

**Priorité : haute** — troisième occurrence de la même famille en une journée
**Contexte :** suite e2e · **Sans épic** · **Voisine des cartes 489 et 496**

## Le constat

`test_correction_ramene_le_rapport_en_etat_corrigeable` ouvre le récapitulatif
d'un rapport publié et attend un bouton « Corriger » actif. Il l'a trouvé
**désactivé**, sur une suite en 7 min 46 — mes deux runs verts de la veille
tenaient en 6 min 05 et 6 min 13, ce test passant à chaque fois.

## La cause

`_play_and_publish` attend que le **rapport** soit `Published` :

```python
publish(space_id, mr_id)
_wait(lambda: _phase(mr_id) == "Published", "le rapport doit être publié")
```

Mais la correction n'est permise que si **les deux camps** sont encore en phase
d'amélioration — `CorrectionBlocker::PhaseAdvanced`. Or l'entrée d'une équipe en
`PlayerImprovement` est le fait d'un listener cross-BC, dans une autre tâche : le
rapport est publié avant que les équipes aient bougé.

Le verdict tombe alors sur un camp encore en `MatchReporting`, et le bouton est
rendu désactivé. Playwright a réessayé quatorze fois sur cinq secondes ; la
transition n'est pas venue.

Le domaine échoue **fermé**, et c'est bien : « autoriser une correction qui
aurait dû être refusée laisserait des données incohérentes, alors qu'un refus
indu ne fait que retarder » (`CorrectionBlocker::EligibilityUnknown`). Ce n'est
pas la prudence qui est en cause, c'est le test qui interroge trop tôt.

## Le fichier connaissait pourtant le remède

Trente lignes plus bas, `test_bouton_desactive_quand_une_equipe_a_valide_sa_phase`
fait exactement ce qu'il faut :

```python
_wait(lambda: _team_phase(home_team) == "PlayerImprovement",
      "l'équipe doit passer en amélioration")
```

**La correction n'avait pas été généralisée.** C'est mot pour mot le constat de
la carte 489 sur `attendre_cablage`, et celui de la 496 sur les attentes de bus.

## La correction

L'attente rejoint `_play_and_publish`, et non le seul test qui tombe : cette
fonction est appelée par **tous** les tests du module, et attendre la phase après
publication est vrai pour chacun. La traiter cas par cas, c'est ce qui a fait
réapparaître la même course trois fois.

**Les deux camps, pas seulement le domicile** : le blocage est déclenché par l'un
ou par l'autre, et n'attendre que l'un laisserait la moitié de la course ouverte.

## Le constat qui dépasse cette carte

**Trois occurrences de la même famille en une journée**, toutes de la forme « le
test lit un état qui dépend d'un traitement asynchrone » :

| Carte | Ce qui n'était pas attendu |
|---|---|
| 489 | le câblage htmx d'un panneau injecté |
| 496 | la projection alimentée par le bus d'app events |
| **499** | la transition de phase d'équipe après publication |

Et **trois copies du même helper** vivent dans trois fichiers : `_wait_for` dans
`test_player_spp_spending`, `_wait_status` dans
`test_player_availability_after_injury`, `_wait` ici — plus `attendre_que`, que
la carte 496 vient d'ajouter dans `db_helpers`. La 496 avait écarté la
convergence « à faire quand un quatrième apparaîtra » : **il est apparu.**

Cette carte ne la fait pas non plus — elle toucherait des tests qui passent, et
la course du jour se ferme sans elle. Mais le compte y est, et la prochaine
occurrence devrait s'appeler « mutualiser les attentes », pas « corriger un
quatrième test ».

## Checklist

- [x] L'attente des deux phases d'équipe dans `_play_and_publish`, donc partagée
      par les six tests du module
- [x] `make lint`, `make check-arch` (17 axes), `make test` (1648),
      `make e2e` (**356 passés**, suite complète 67/67, 0 échec)

## L'hypothèse concurrente, écartée

Un bouton désactivé pouvait aussi venir de `SppAlreadySpent` : un test voisin
ayant dépensé des SPP sur les mêmes équipes bloquerait la correction, et ce
serait déterministe, pas une course.

**Vérifié et faux.** Chaque test du module a sa propre paire — 0/1, 2/3, 4/5,
6/7, 8/9, 10/11 — et celui qui tombe est le **premier**. Aucune contamination
possible. Le blocage ne peut venir que de la phase, et la phase arrive par le
bus.

## Terminé quand

La suite passe sur une machine chargée. **Ce que la carte ne pourra pas
démontrer** : que le correctif attrape la course — elle ne s'ouvre que sous
charge, et une machine déchargée rend la transition déjà faite au retour de la
publication.
