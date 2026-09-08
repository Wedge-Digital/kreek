# L'exemption va à celle qui a le plus joué

**Priorité : haute — la 517 écrit le tirage du sondage, elle doit le trouver juste**
**Épic :** E16 — Sondage de présence, vague 1 « corriger l'existant »
**Dépend de :** 507, qui a réécrit le moteur
**Bloque :** 517 — le tirage du sondage appelle `tirer` et remplirait sinon deux
fois le même `DrawInput`
**Fichiers :** `src/app/competitions/domain/tirage.rs`,
`src/app/competitions/use_cases/admin/generate_pairings.rs`,
`docs/specs/sondage-presence/README.md`

## Le problème

R9 dit aujourd'hui : *nombre impair de présents ⇒ une équipe est exemptée, tirée
parmi celles qui ne l'ont jamais été sur la saison.*

**Cette règle est inerte en production.** `generate_pairings.rs:220` passe
`jamais_exemptees: HashSet::new()`. Avec un ensemble vide, `tirage.rs:340` marque
**toutes** les équipes comme déjà exemptées, `cout_exemption` rend le même coût
pour chacune, et le critère ne départage rien. Elle est implémentée dans le
moteur, testée dans le moteur, et n'a jamais été alimentée — parce que personne
n'avait de source pour « qui n'a jamais été exempté ».

Et le critère lui-même mesure la mauvaise chose. Il égalise le nombre
d'exemptions ; ce qui se vit comme une injustice, c'est le nombre de matchs. Les
deux divergent dès qu'une équipe manque des journées en se déclarant absente :
elle n'a jamais été exemptée — donc R9 l'exempte en priorité — alors qu'elle a
le moins joué. Dans une ligue où les présences sont par construction
irrégulières, c'est exactement le cas qu'il ne faut pas rater.

## La règle qui remplace

**L'exemption va à l'équipe qui compte le plus d'appariements sur la saison.**
Simple, et elle se calcule à partir de ce que le BC possède déjà.

Les **appariements programmés**, joués ou non — pas les rapports de match.
Compter les matchs réellement joués demanderait d'interroger `match_report` par
un port, pour une différence qui ne concerne que les rencontres reportées.
`build_historique` parcourt déjà exactement ces appariements pour compter les
rencontres **par paire** ; compter **par équipe** est la même boucle.

## Ce que ça change

| Aujourd'hui | Après |
|---|---|
| `DrawInput.jamais_exemptees: HashSet<TeamId>` | `DrawInput.matchs_joues: HashMap<TeamId, NombreDeMatchs>` |
| `Recherche.deja_exemptee: Vec<bool>` | `Recherche.retard: Vec<u32>` — l'écart au maximum |
| `Cout.exemptions_repetees` — 1 si déjà exemptée | `Cout.exemption_injuste` — le retard de l'exemptée |
| `jamais_exemptees: HashSet::new()` | le compte, dans la boucle de `build_historique` |

`NombreDeMatchs` est un newtype, comme `NombreDeRencontres` du même fichier.

**Le coût reste au quatrième rang** de la comparaison lexicographique. R8.1 puis
R8.2 primaient sur R9 ; ils priment sur son remplaçant. Préférer une rencontre
inédite à l'exemption souhaitée reste vrai — c'est ce que fixe
`r9_cede_devant_le_nombre_de_revanches`, à réécrire sur le nouveau critère et
**non à supprimer** : c'est le seul test qui tient la hiérarchie des objectifs.

**Le retard, et non le compte brut.** `Cout` s'additionne le long de la
recherche (`Cout::plus`) et se compare à un plancher ; une dimension qui porte
un nombre de matchs absolu ferait dépendre l'élagage de l'avancement de la
saison, et un plancher calculé à la journée 1 ne vaudrait plus rien à la
journée 15. Le retard au maximum vaut zéro pour le meilleur choix, quelle que
soit la journée.

## Ce que ça change pour le Calendrier

Le même `tirer` sert les deux chemins : **le Calendrier hérite du critère**, et
c'est voulu. C'est un changement de comportement sur une fonction existante, pas
seulement sur le sondage — d'où la vague 1 de l'épic, comme 507, 508 et 509.

En pratique il n'y a rien à perdre : le critère n'y départageait rien, faute
d'être alimenté. Le Calendrier passe donc d'un tirage où l'exemptée est un pur
hasard à un tirage où elle est la plus servie.

## Ce que ça ne change pas

Rien dans `PresenceSurvey` : `Appariement::Fait { exemptee }` garde l'exemptée
qui a **finalement** eu lieu (R9 croisée à R12, carte 513), et cette carte ne
touche pas à ce qu'on en fait — seulement à la façon de la choisir.

## Mise à jour de la spec

R9 est réécrite dans `docs/specs/sondage-presence/README.md`, avec la trace de
ce qui a changé et pourquoi : le critère « jamais exemptée » n'était pas
mesurable avec les données du BC, celui-ci l'est. Une règle qu'on ne peut pas
alimenter n'est pas une règle, c'est une intention.

## Checklist

- [ ] `NombreDeMatchs`, `DrawInput.matchs_joues`
- [ ] `Cout.exemption_injuste` au rang 4, et `plancher()` mis d'accord
- [ ] `build_historique` rend aussi le compte par équipe
- [ ] `generate_pairings.rs` alimente vraiment le champ
- [ ] R9 réécrite dans la spec
- [ ] Tests : l'exemptée est celle du maximum · à égalité, R17 départage au sort ·
      le critère ne prime ni sur R8.1 ni sur R8.2 (`r9_cede_devant_…` réécrit) ·
      l'appelant du Calendrier alimente le compte — le défaut d'aujourd'hui, qu'un
      test aurait vu
- [ ] `make lint`, `make check-arch`, `make test`, `make test-impacted`
