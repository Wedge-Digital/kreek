# « Publié par » au récapitulatif

**Épic :** E19 — Saisie du compte rendu : droits et robustesse · **Ordre :** 4 · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E19-cadrage.md` (R5)

## Objectif

Le récapitulatif nomme son auteur en lisant `created_by`. Pour tout brouillon
issu du calendrier, ce champ vaut l'identifiant de la compétition : aucun
coach ne le porte, et `build_submitted_by` masque la ligne. Après la carte, le
récapitulatif d'un compte rendu publié nomme **celui qui l'a publié**.

## Conception

`RecapSource` lit `published_by` (état `Published`, événement
`MatchReportPublished`) au lieu de `created_by`. Un compte rendu prêt à
publier, jamais publié, n'a pas de publieur : la ligne reste absente. Le
libellé devient « Publié par ».

**`created_by` n'est pas touché** : écrit dans l'événement `MatchReportCreated`
(immuable) et dans `match_report_proj.created_by`, il garde son sens actuel.
Le changer donnerait deux sens à la même donnée selon sa date.

## Données de production

Aucune réécrite. Tous les récapitulatifs publiés, passés compris, affichent
désormais leur publieur — ceux issus du calendrier n'affichaient rien.

## Checklist

- [ ] `RecapSource` et le gabarit lisent `published_by`
- [ ] Test unitaire : un rapport publié nomme son publieur, un rapport non publié n'en nomme pas
- [ ] E2E : le récapitulatif publié affiche « Publié par » et le coach
- [ ] `make lint`, `make check-arch`, `make test`, e2e impactés
