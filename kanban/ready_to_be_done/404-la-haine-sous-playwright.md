# La Haine sous Playwright

**Priorité : haute**
**Dépend de :** 402 et 403
**Conception :** `docs/specs/haine/saisie-des-actions/07-integration.md`
**Fichiers :** `tests/e2e/`, `.claude/skills/test-impact/` (carte d'impact)

## Objectif

Les huit scénarios de la page, dont deux que seul un navigateur peut voir.

| # | Scénario | Vérifie |
|---|---|---|
| 1 | Amoché → section visible, réponse Non → action enregistrée sans Haine | R1, R2 |
| 2 | Séquelle → Oui → mot-clef → le journal affiche la Haine | chemin nominal |
| 3 | Commotion → **la section n'apparaît pas** | R1 côté front |
| 4 | Oui sans mot-clef → **la confirmation reste masquée** | R3 côté front |
| 5 | Filtrer « yéti » → le repli s'ouvre seul, le mot apparaît | ergonomie |
| 6 | Mot-clef choisi, puis filtre qui ne le contient pas → il reste visible | ergonomie |
| 7 | Deux fois le même mot-clef sur un joueur → **accepté** | R7 |
| 8 | Journalier blessé avec Haine → publier → le joueur permanent est intact | R9 |

## Les scénarios 3 et 4 sont la raison d'être de cette carte

Ils vérifient qu'une chose **n'apparaît pas**. Aucun test unitaire ne peut le
faire : la logique serveur est identique, seule la conditionnelle du template
change. Le `CLAUDE.md` le dit du widget coach-search et des pickers de tiers —
« n'auraient été détectés par aucun test unitaire, uniquement par un test E2E
piloté en navigateur ».

## Le huitième demande un rapport publié

C'est le seul scénario long, et le seul qui vérifie une **absence d'écriture
ailleurs** : la Haine d'un journalier reste dans le rapport et ne rejoint aucun
agrégat `players`. Il faut donc mener un match jusqu'à la publication.

À voir à l'implémentation s'il peut réutiliser un parcours existant plutôt que
d'en refaire un : la suite dure déjà 7 min 30, et la carte 312 vise à la
raccourcir.

## Ne pas oublier la carte d'impact

Le skill `test-impact` tient une carte tests ↔ bounded contexts. **Un nouveau
test e2e impose sa mise à jour** — sans quoi il ne sera jamais sélectionné par
les exécutions ciblées, et ne tournera qu'en CI complète.

## Checklist

- [ ] Les huit scénarios, dans `tests/e2e/`
- [ ] Le jeu de données e2e porte un roster adverse aux mots-clefs connus
- [ ] Carte d'impact tests ↔ BC mise à jour
- [ ] Chaque test **vu échouer** avant d'être vu passer — un test qui n'a jamais
      été rouge ne prouve rien
- [ ] `make e2e` complet au vert
